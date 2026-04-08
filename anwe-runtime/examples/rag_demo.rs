// ─────────────────────────────────────────────────────────
// RAG PIPELINE DEMO — ANWE + BRIDGE PARTICIPANTS
//
// This is the Rust side of the RAG pipeline.
// The pipeline itself is defined in rag_pipeline.anwe.
// This file wires up the bridge participants — the external
// systems that the .anwe program talks to through signals.
//
// Three external participants:
//   Embedder   — simulates an embedding model
//   Store      — simulates a vector database
//   Generator  — simulates an LLM
//
// In production, these would bridge to real services.
// Here they simulate realistic behavior to demonstrate
// the bridge protocol working end-to-end.
//
// Run with: cargo run --example rag_demo
// ─────────────────────────────────────────────────────────

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};

use anwe_parser::{Lexer, Parser};
use anwe_runtime::Engine;
use anwe_bridge::{ParticipantRegistry, ParticipantDescriptor, WireSignal, WireValue};
use anwe_bridge::participant::CallbackParticipant;

fn main() {
    println!();
    println!("ANWE v0.1 — RAG Pipeline Demo");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("This is a Retrieval-Augmented Generation pipeline");
    println!("written entirely in ANWE with bridge participants.");
    println!("Not a Primordia feature. Not an agent demo.");
    println!("A real AI pipeline in the language of AI.");
    println!();

    // ─── CREATE BRIDGE PARTICIPANTS ──────────────────────
    let mut registry = ParticipantRegistry::new();

    // Track what each participant sees (for the demo output)
    let embedder_signals = Arc::new(AtomicU32::new(0));
    let store_signals = Arc::new(AtomicU32::new(0));
    let generator_signals = Arc::new(AtomicU32::new(0));
    let committed_stages: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    // ─── EMBEDDER ────────────────────────────────────────
    // Simulates an embedding model (e.g., sentence-transformers).
    // Receives query text signals, responds with "embedding ready."
    {
        let count = Arc::clone(&embedder_signals);
        registry.register("Embedder", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "Embedder".into(),
                kind: "callback".into(),
                address: "embedder".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                // Respond with embedding confirmation
                Some(WireSignal {
                    quality: 4,  // Applying — work is being done
                    direction: 2, // Between
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String(
                        "embedding: [0.23, -0.14, 0.87, ...] (384 dims)".into()
                    )),
                    confidence: 0.95,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,  // Accept all structural changes
            |_entries| {},
        )));
    }

    // ─── VECTOR STORE ────────────────────────────────────
    // Simulates a vector database (e.g., FAISS, Pinecone).
    // Receives search signals, responds with match counts.
    {
        let count = Arc::clone(&store_signals);
        registry.register("Store", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "VectorStore".into(),
                kind: "callback".into(),
                address: "vectorstore".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                // Respond with retrieval results
                Some(WireSignal {
                    quality: 2,  // Recognizing — patterns found
                    direction: 2, // Between
                    priority: signal.priority * 0.9,
                    data: Some(WireValue::Map(vec![
                        ("matches".into(), WireValue::Integer(10)),
                        ("top_score".into(), WireValue::Float(0.89)),
                        ("search_ms".into(), WireValue::Float(12.3)),
                    ])),
                    confidence: 0.85,
                    half_life: 500, // Results decay over time
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            |_entries| {},
        )));
    }

    // ─── GENERATOR ───────────────────────────────────────
    // Simulates an LLM (e.g., GPT-4, Claude).
    // Receives context signals, responds with generation status.
    {
        let count = Arc::clone(&generator_signals);
        let stages = Arc::clone(&committed_stages);
        registry.register("Generator", Box::new(CallbackParticipant::new(
            ParticipantDescriptor {
                name: "Generator".into(),
                kind: "callback".into(),
                address: "llm".into(),
                version: "0.1.0".into(),
            },
            move |signal: &WireSignal| {
                count.fetch_add(1, Ordering::Relaxed);
                // Respond with generation in progress
                Some(WireSignal {
                    quality: 4,  // Applying — generating
                    direction: 2, // Between
                    priority: signal.priority * 0.95,
                    data: Some(WireValue::String(
                        "generating: attention mechanisms allow models to dynamically weigh...".into()
                    )),
                    confidence: 0.88,
                    half_life: 0,
                    sequence: signal.sequence + 1,
                })
            },
            |_changes| true,
            move |entries| {
                // Record committed stages
                for (key, val) in entries {
                    if key == "stage" {
                        if let WireValue::String(s) = val {
                            stages.lock().unwrap().push(s.clone());
                        }
                    }
                }
            },
        )));
    }

    // ─── PARSE THE PIPELINE ──────────────────────────────
    let source = include_str!("rag_pipeline.anwe");

    let mut lexer = Lexer::new(source);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lex error: {}", e);
            std::process::exit(1);
        }
    };

    let mut parser = Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            std::process::exit(1);
        }
    };

    // ─── EXECUTE ─────────────────────────────────────────
    println!("Executing rag_pipeline.anwe with bridge participants...");
    println!("═══════════════════════════════════════════════");

    let mut engine = Engine::with_participants(registry);
    match engine.execute(&program) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Engine error: {}", e);
            std::process::exit(1);
        }
    }

    // ─── RESULTS ─────────────────────────────────────────
    println!();
    println!("═══════════════════════════════════════════════");
    println!("BRIDGE PARTICIPANT ACTIVITY");
    println!("═══════════════════════════════════════════════");
    println!();
    println!("  Embedder   received {} signals", embedder_signals.load(Ordering::Relaxed));
    println!("  Store      received {} signals", store_signals.load(Ordering::Relaxed));
    println!("  Generator  received {} signals", generator_signals.load(Ordering::Relaxed));
    println!();

    let stages = committed_stages.lock().unwrap();
    if !stages.is_empty() {
        println!("  Committed stages: {:?}", *stages);
    }

    println!();
    println!("This pipeline was defined in ~150 lines of ANWE.");
    println!("The Python equivalent requires ~200+ lines and");
    println!("reimplements confidence, temporal decay, attention");
    println!("budgets, and pending states from scratch.");
    println!();
}
