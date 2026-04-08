// -----------------------------------------------------------------
// ANWE v0.1 -- STDIO PARTICIPANT
//
// Bridges an external process as an ANWE participant via
// JSON over stdin/stdout.
//
// Protocol:
//   Runtime sends JSON objects to the process's stdin:
//     {"type": "receive", "signal": {...}}
//     {"type": "apply", "changes": {...}}
//     {"type": "commit", "entries": {...}}
//
//   Process responds on stdout with JSON:
//     {"response": {...}} or {"response": null}
//     {"accept": true/false}
//     (no response needed for commit)
//
// This allows any language (Python, Node, Ruby, Go, etc.)
// to participate in ANWE coordination by implementing a
// simple JSON protocol over stdio.
// -----------------------------------------------------------------

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

use crate::participant::{Participant, ParticipantDescriptor};
use crate::wire::{WireSignal, WireValue};

/// A participant that communicates with an external process
/// via JSON over stdin/stdout.
pub struct StdioParticipant {
    desc: ParticipantDescriptor,
    child: Child,
}

impl StdioParticipant {
    /// Spawn a new stdio participant from a command.
    ///
    /// The command is split on `:` — the first part is the protocol
    /// (currently only "cmd" is supported), the rest is the command.
    ///
    /// Example: `cmd:python3 my_agent.py`
    pub fn spawn(agent_name: &str, spec: &str) -> Result<Self, String> {
        let cmd_str = if let Some(rest) = spec.strip_prefix("cmd:") {
            rest
        } else {
            spec
        };

        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty command".into());
        }

        let child = Command::new(parts[0])
            .args(&parts[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("Failed to spawn {}: {}", cmd_str, e))?;

        Ok(StdioParticipant {
            desc: ParticipantDescriptor {
                name: agent_name.to_string(),
                kind: "stdio".to_string(),
                address: cmd_str.to_string(),
                version: "0.1.0".to_string(),
            },
            child,
        })
    }

    fn send_recv(&mut self, msg: &str) -> Option<String> {
        let stdin = self.child.stdin.as_mut()?;
        writeln!(stdin, "{}", msg).ok()?;
        stdin.flush().ok()?;

        let stdout = self.child.stdout.as_mut()?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;

        if line.trim().is_empty() {
            None
        } else {
            Some(line.trim().to_string())
        }
    }
}

impl Drop for StdioParticipant {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

// Safety: StdioParticipant owns its child process exclusively
unsafe impl Send for StdioParticipant {}

impl Participant for StdioParticipant {
    fn receive(&mut self, signal: &WireSignal) -> Option<WireSignal> {
        let msg = format!(
            r#"{{"type":"receive","signal":{{"quality":{},"direction":{},"priority":{},"confidence":{},"half_life":{},"sequence":{}}}}}"#,
            signal.quality, signal.direction, signal.priority,
            signal.confidence, signal.half_life, signal.sequence
        );

        let response = self.send_recv(&msg)?;

        // Parse minimal JSON response
        if response.contains("null") || response.is_empty() {
            return None;
        }

        // Simple JSON parsing for the response signal
        let quality = parse_json_u8(&response, "quality").unwrap_or(signal.quality);
        let direction = parse_json_u8(&response, "direction").unwrap_or(signal.direction);
        let priority = parse_json_f32(&response, "priority").unwrap_or(signal.priority * 0.9);
        let confidence = parse_json_f32(&response, "confidence").unwrap_or(signal.confidence);

        Some(WireSignal {
            quality,
            direction,
            priority,
            data: None,
            confidence,
            half_life: signal.half_life,
            sequence: signal.sequence + 1,
        })
    }

    fn apply(&mut self, changes: &[(String, WireValue)]) -> bool {
        let pairs: Vec<String> = changes.iter()
            .map(|(k, v)| format!(r#""{}":"{}""#, k, wire_value_to_json(v)))
            .collect();
        let msg = format!(r#"{{"type":"apply","changes":{{{}}}}}"#, pairs.join(","));

        match self.send_recv(&msg) {
            Some(resp) => !resp.contains("false"),
            None => true,
        }
    }

    fn commit(&mut self, entries: &[(String, WireValue)]) {
        let pairs: Vec<String> = entries.iter()
            .map(|(k, v)| format!(r#""{}":"{}""#, k, wire_value_to_json(v)))
            .collect();
        let msg = format!(r#"{{"type":"commit","entries":{{{}}}}}"#, pairs.join(","));
        let _ = self.send_recv(&msg);
    }

    fn descriptor(&self) -> &ParticipantDescriptor {
        &self.desc
    }
}

fn wire_value_to_json(val: &WireValue) -> String {
    match val {
        WireValue::String(s) => s.clone(),
        WireValue::Integer(i) => i.to_string(),
        WireValue::Float(f) => f.to_string(),
        WireValue::Bool(b) => b.to_string(),
        WireValue::Null => "null".to_string(),
        WireValue::Bytes(b) => format!("{:?}", b),
        WireValue::List(_) => "[...]".to_string(),
        WireValue::Map(_) => "{...}".to_string(),
    }
}

fn parse_json_u8(json: &str, key: &str) -> Option<u8> {
    let pattern = format!("\"{}\":", key);
    let pos = json.find(&pattern)?;
    let rest = &json[pos + pattern.len()..];
    let rest = rest.trim_start();
    let end = rest.find(|c: char| !c.is_ascii_digit())?;
    rest[..end].parse().ok()
}

fn parse_json_f32(json: &str, key: &str) -> Option<f32> {
    let pattern = format!("\"{}\":", key);
    let pos = json.find(&pattern)?;
    let rest = &json[pos + pattern.len()..];
    let rest = rest.trim_start();
    let end = rest.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}
