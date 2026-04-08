// -----------------------------------------------------------------
// ANWE v0.1 -- PYTHON BINDINGS
//
// PyO3 bridge between ANWE and Python.
//
// This crate exposes the ANWE Participant protocol to Python.
// Any Python class that implements receive(), apply(), commit(),
// and descriptor() can participate in ANWE signal exchange.
//
// The Rust runtime calls Python through PyO3. Python doesn't
// need to know about cache lines, atomics, or fiber scheduling.
// It just gets WireSignals and responds.
//
// Architecture:
//   Python class → PyParticipant (Rust wrapper) → Participant trait
//   WireSignal ↔ PyWireSignal (Python-friendly representation)
//   WireValue ↔ Python native types (str, int, float, list, dict)
// -----------------------------------------------------------------

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString, PyBool, PyFloat, PyInt};

use anwe_bridge::{
    Participant, ParticipantDescriptor,
    WireSignal, WireValue,
};

// -----------------------------------------------------------------
// PyWireSignal — Python-visible signal representation
// -----------------------------------------------------------------

/// A signal as seen from Python.
///
/// Python code receives these and can create them as responses.
/// Quality and direction are integer codes (see ANWE spec).
#[pyclass(name = "WireSignal")]
pub struct PyWireSignal {
    #[pyo3(get, set)]
    pub quality: u8,
    #[pyo3(get, set)]
    pub direction: u8,
    #[pyo3(get, set)]
    pub priority: f32,
    #[pyo3(get, set)]
    pub data: PyObject,
    #[pyo3(get, set)]
    pub confidence: f32,
    #[pyo3(get, set)]
    pub half_life: u16,
    #[pyo3(get, set)]
    pub sequence: u64,
}

impl Clone for PyWireSignal {
    fn clone(&self) -> Self {
        Python::with_gil(|py| {
            PyWireSignal {
                quality: self.quality,
                direction: self.direction,
                priority: self.priority,
                data: self.data.clone_ref(py),
                confidence: self.confidence,
                half_life: self.half_life,
                sequence: self.sequence,
            }
        })
    }
}

#[pymethods]
impl PyWireSignal {
    #[new]
    #[pyo3(signature = (quality=0, direction=2, priority=0.5, data=None, confidence=1.0, half_life=0, sequence=0))]
    fn new(
        py: Python<'_>,
        quality: u8,
        direction: u8,
        priority: f32,
        data: Option<PyObject>,
        confidence: f32,
        half_life: u16,
        sequence: u64,
    ) -> Self {
        PyWireSignal {
            quality,
            direction,
            priority,
            data: data.unwrap_or_else(|| py.None().into()),
            confidence,
            half_life,
            sequence,
        }
    }

    /// Quality name as string.
    #[getter]
    fn quality_name(&self) -> &'static str {
        match self.quality {
            0 => "attending",
            1 => "questioning",
            2 => "recognizing",
            3 => "disturbed",
            4 => "applying",
            5 => "completing",
            _ => "resting",
        }
    }

    /// Direction name as string.
    #[getter]
    fn direction_name(&self) -> &'static str {
        match self.direction {
            0 => "inward",
            1 => "outward",
            2 => "between",
            _ => "diffuse",
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "WireSignal(quality={}, direction={}, priority={:.3}, confidence={:.3})",
            self.quality_name(), self.direction_name(),
            self.priority, self.confidence,
        )
    }
}

// -----------------------------------------------------------------
// PyParticipantDescriptor — Python-visible metadata
// -----------------------------------------------------------------

/// Metadata about a participant.
#[pyclass(name = "ParticipantDescriptor")]
#[derive(Clone)]
pub struct PyParticipantDescriptor {
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub kind: String,
    #[pyo3(get, set)]
    pub address: String,
    #[pyo3(get, set)]
    pub version: String,
}

#[pymethods]
impl PyParticipantDescriptor {
    #[new]
    #[pyo3(signature = (name, kind="python".to_string(), address="".to_string(), version="0.1.0".to_string()))]
    fn new(name: String, kind: String, address: String, version: String) -> Self {
        PyParticipantDescriptor { name, kind, address, version }
    }

    fn __repr__(&self) -> String {
        format!("ParticipantDescriptor(name='{}', kind='{}')", self.name, self.kind)
    }
}

// -----------------------------------------------------------------
// CONVERSION: WireSignal ↔ PyWireSignal
// -----------------------------------------------------------------

impl PyWireSignal {
    /// Convert from Rust WireSignal to Python WireSignal.
    fn from_wire(py: Python<'_>, wire: &WireSignal) -> Self {
        let data = wire_value_to_py(py, &wire.data);
        PyWireSignal {
            quality: wire.quality,
            direction: wire.direction,
            priority: wire.priority,
            data,
            confidence: wire.confidence,
            half_life: wire.half_life,
            sequence: wire.sequence,
        }
    }

    /// Convert from Python WireSignal to Rust WireSignal.
    fn to_wire(&self, py: Python<'_>) -> WireSignal {
        WireSignal {
            quality: self.quality,
            direction: self.direction,
            priority: self.priority,
            data: py_to_wire_value(py, &self.data),
            confidence: self.confidence,
            half_life: self.half_life,
            sequence: self.sequence,
        }
    }
}

// -----------------------------------------------------------------
// CONVERSION: WireValue ↔ Python native types
//
// WireValue::String  ↔ str
// WireValue::Integer ↔ int
// WireValue::Float   ↔ float
// WireValue::Bool    ↔ bool
// WireValue::Null    ↔ None
// WireValue::List    ↔ list
// WireValue::Map     ↔ dict
// WireValue::Bytes   ↔ bytes
// -----------------------------------------------------------------

fn wire_value_to_py(py: Python<'_>, val: &Option<WireValue>) -> PyObject {
    match val {
        None => py.None().into(),
        Some(WireValue::Null) => py.None().into(),
        Some(WireValue::Bool(b)) => PyBool::new(py, *b).to_owned().into_any().unbind(),
        Some(WireValue::Integer(i)) => i.into_pyobject(py).unwrap().into_any().unbind(),
        Some(WireValue::Float(f)) => f.into_pyobject(py).unwrap().into_any().unbind(),
        Some(WireValue::String(s)) => PyString::new(py, s).into_any().unbind(),
        Some(WireValue::Bytes(b)) => b.as_slice().into_pyobject(py).unwrap().into_any().unbind(),
        Some(WireValue::List(items)) => {
            let py_list = PyList::empty(py);
            for item in items {
                let py_item = wire_value_to_py(py, &Some(item.clone()));
                py_list.append(py_item).unwrap();
            }
            py_list.into_any().unbind()
        }
        Some(WireValue::Map(entries)) => {
            let py_dict = PyDict::new(py);
            for (key, val) in entries {
                let py_val = wire_value_to_py(py, &Some(val.clone()));
                py_dict.set_item(key, py_val).unwrap();
            }
            py_dict.into_any().unbind()
        }
    }
}

fn py_to_wire_value(py: Python<'_>, obj: &PyObject) -> Option<WireValue> {
    let bound = obj.bind(py);

    if bound.is_none() {
        return None;
    }
    if let Ok(b) = bound.downcast::<PyBool>() {
        return Some(WireValue::Bool(b.is_true()));
    }
    if let Ok(i) = bound.downcast::<PyInt>() {
        if let Ok(v) = i.extract::<i64>() {
            return Some(WireValue::Integer(v));
        }
    }
    if let Ok(f) = bound.downcast::<PyFloat>() {
        if let Ok(v) = f.extract::<f64>() {
            return Some(WireValue::Float(v));
        }
    }
    if let Ok(s) = bound.downcast::<PyString>() {
        if let Ok(v) = s.extract::<String>() {
            return Some(WireValue::String(v));
        }
    }
    if let Ok(list) = bound.downcast::<PyList>() {
        let mut items = Vec::new();
        for item in list.iter() {
            if let Some(wv) = py_to_wire_value(py, &item.unbind()) {
                items.push(wv);
            } else {
                items.push(WireValue::Null);
            }
        }
        return Some(WireValue::List(items));
    }
    if let Ok(dict) = bound.downcast::<PyDict>() {
        let mut entries = Vec::new();
        for (key, val) in dict.iter() {
            if let Ok(k) = key.extract::<String>() {
                let wv = py_to_wire_value(py, &val.unbind())
                    .unwrap_or(WireValue::Null);
                entries.push((k, wv));
            }
        }
        return Some(WireValue::Map(entries));
    }

    // Fallback: convert to string
    if let Ok(s) = bound.str() {
        if let Ok(v) = s.extract::<String>() {
            return Some(WireValue::String(v));
        }
    }

    None
}

fn wire_changes_to_py(py: Python<'_>, changes: &[(String, WireValue)]) -> PyObject {
    let dict = PyDict::new(py);
    for (key, val) in changes {
        let py_val = wire_value_to_py(py, &Some(val.clone()));
        dict.set_item(key, py_val).unwrap();
    }
    dict.into_any().unbind()
}

// -----------------------------------------------------------------
// PyParticipant — wraps a Python object as a Rust Participant
//
// This is the bridge. A Python class that has receive(), apply(),
// commit(), and descriptor() methods can be registered with the
// ANWE runtime. The Rust trait methods call into Python through
// the GIL.
// -----------------------------------------------------------------

/// Wraps a Python object that implements the Participant protocol.
///
/// The Python object must have these methods:
///   def receive(self, signal: WireSignal) -> Optional[WireSignal]
///   def apply(self, changes: dict) -> bool
///   def commit(self, entries: dict) -> None
///   def descriptor(self) -> ParticipantDescriptor
///
/// Optional:
///   def attention(self) -> float  (default 1.0)
pub struct PyParticipant {
    py_obj: PyObject,
    desc: ParticipantDescriptor,
}

impl PyParticipant {
    /// Create a new PyParticipant from a Python object.
    pub fn new(py: Python<'_>, obj: PyObject) -> PyResult<Self> {
        // Extract descriptor from the Python object
        let desc_obj = obj.call_method0(py, "descriptor")?;
        let desc_bound = desc_obj.bind(py);

        let name: String = desc_bound.getattr("name")?.extract()?;
        let kind: String = desc_bound.getattr("kind")?.extract()?;
        let address: String = desc_bound.getattr("address")?.extract()?;
        let version: String = desc_bound.getattr("version")?.extract()?;

        Ok(PyParticipant {
            py_obj: obj,
            desc: ParticipantDescriptor { name, kind, address, version },
        })
    }
}

// Safety: PyO3 objects can be sent between threads when we hold the GIL
unsafe impl Send for PyParticipant {}

impl Participant for PyParticipant {
    fn receive(&mut self, signal: &WireSignal) -> Option<WireSignal> {
        Python::with_gil(|py| {
            let py_signal = PyWireSignal::from_wire(py, signal);
            let py_signal_obj = Py::new(py, py_signal).ok()?;

            let result = self.py_obj
                .call_method1(py, "receive", (py_signal_obj,))
                .ok()?;

            if result.bind(py).is_none() {
                return None;
            }

            // Extract the response WireSignal
            let response: PyRef<'_, PyWireSignal> = result.bind(py).extract().ok()?;
            Some(response.to_wire(py))
        })
    }

    fn apply(&mut self, changes: &[(String, WireValue)]) -> bool {
        Python::with_gil(|py| {
            let py_changes = wire_changes_to_py(py, changes);

            self.py_obj
                .call_method1(py, "apply", (py_changes,))
                .ok()
                .and_then(|r| r.extract::<bool>(py).ok())
                .unwrap_or(true)
        })
    }

    fn commit(&mut self, entries: &[(String, WireValue)]) {
        Python::with_gil(|py| {
            let py_entries = wire_changes_to_py(py, entries);
            let _ = self.py_obj.call_method1(py, "commit", (py_entries,));
        })
    }

    fn attention(&self) -> f32 {
        Python::with_gil(|py| {
            self.py_obj
                .call_method0(py, "attention")
                .ok()
                .and_then(|r| r.extract::<f32>(py).ok())
                .unwrap_or(1.0)
        })
    }

    fn descriptor(&self) -> &ParticipantDescriptor {
        &self.desc
    }
}

// -----------------------------------------------------------------
// Python module: anwe_python
//
// This is what Python imports:
//   from anwe_python import WireSignal, ParticipantDescriptor
// -----------------------------------------------------------------

/// ANWE Python bindings — participate in ANWE signal exchange from Python.
#[pymodule]
fn anwe_python(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWireSignal>()?;
    m.add_class::<PyParticipantDescriptor>()?;

    // Signal quality constants
    m.add("ATTENDING", 0u8)?;
    m.add("QUESTIONING", 1u8)?;
    m.add("RECOGNIZING", 2u8)?;
    m.add("DISTURBED", 3u8)?;
    m.add("APPLYING", 4u8)?;
    m.add("COMPLETING", 5u8)?;
    m.add("RESTING", 6u8)?;

    // Direction constants
    m.add("INWARD", 0u8)?;
    m.add("OUTWARD", 1u8)?;
    m.add("BETWEEN", 2u8)?;
    m.add("DIFFUSE", 3u8)?;

    Ok(())
}
