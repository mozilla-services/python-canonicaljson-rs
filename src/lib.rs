use canonical_json::ser::{to_string, CanonicalJSONError};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyFloat, PyList, PyTuple};
use pyo3::wrap_pyfunction;
use serde_json::Value as JsonValue;

pub enum PyCanonicalJSONError {
    InvalidConversion { error: String },
    PyErr { error: String },
    DictKeyNotSerializable { typename: String },
    InvalidFloat { typename: String },
    InvalidCast { typename: String },
}

impl From<CanonicalJSONError> for PyCanonicalJSONError {
    fn from(error: CanonicalJSONError) -> Self {
        PyCanonicalJSONError::InvalidConversion {
            error: format!("{:?}", error),
        }
    }
}

impl From<pyo3::PyErr> for PyCanonicalJSONError {
    fn from(error: pyo3::PyErr) -> Self {
        PyCanonicalJSONError::PyErr {
            error: format!("{:?}", error),
        }
    }
}

impl From<PyCanonicalJSONError> for pyo3::PyErr {
    fn from(e: PyCanonicalJSONError) -> pyo3::PyErr {
        match e {
            PyCanonicalJSONError::InvalidConversion { error } => {
                PyErr::new::<PyTypeError, _>(format!("Conversion error: {error}"))
            }
            PyCanonicalJSONError::PyErr { error } => {
                PyErr::new::<PyTypeError, _>(format!("Python Runtime exception: {error}"))
            }
            PyCanonicalJSONError::DictKeyNotSerializable { typename } => {
                PyErr::new::<PyTypeError, _>(format!(
                    "Dictionary key is not serializable: {typename}"
                ))
            }
            PyCanonicalJSONError::InvalidFloat { typename } => {
                PyErr::new::<PyTypeError, _>(format!("Invalid float (NaN/Inf): type {typename}"))
            }
            PyCanonicalJSONError::InvalidCast { typename } => {
                PyErr::new::<PyTypeError, _>(format!("Invalid type: {typename}"))
            }
        }
    }
}

/// A canonical JSON serializer written in Rust
#[pymodule]
fn canonicaljson(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_wrapped(wrap_pyfunction!(dump))?;
    m.add_wrapped(wrap_pyfunction!(dumps))?;
    Ok(())
}

#[pyfunction]
pub fn dump(py: Python, obj: Py<PyAny>, fp: Py<PyAny>) -> PyResult<Py<PyAny>> {
    let s_obj = dumps(py, obj)?; // Py<PyAny> (str)
    fp.bind(py).call_method1("write", (s_obj.bind(py),))?;
    Ok(py.None()) // already Py<PyAny>
}

#[pyfunction]
pub fn dumps(py: Python, obj: Py<PyAny>) -> PyResult<Py<PyAny>> {
    let v = to_json(py, &obj)?;
    match to_string(&v) {
        Ok(s) => Ok(s.into_pyobject(py)?.unbind().into()), // Py<PyString> -> Py<PyAny>
        Err(e) => Err(PyErr::new::<PyTypeError, _>(format!("{:?}", e))),
    }
}

fn type_name(any: &Bound<'_, PyAny>) -> PyResult<String> {
    Ok(any.get_type().name()?.to_str()?.to_string())
}

fn to_json(py: Python, obj: &Py<PyAny>) -> Result<JsonValue, PyCanonicalJSONError> {
    let any = obj.bind(py);

    // None -> JSON null
    if any.is_none() {
        return Ok(JsonValue::Null);
    }

    // Primitive extracts
    if let Ok(s) = any.extract::<String>() {
        return Ok(JsonValue::String(s));
    }
    if let Ok(b) = any.extract::<bool>() {
        return Ok(JsonValue::Bool(b));
    }
    if let Ok(u) = any.extract::<u64>() {
        return Ok(serde_json::value::to_value(u).map_err(|e| {
            PyCanonicalJSONError::InvalidConversion {
                error: e.to_string(),
            }
        })?);
    }
    if let Ok(i) = any.extract::<i64>() {
        return Ok(serde_json::value::to_value(i).map_err(|e| {
            PyCanonicalJSONError::InvalidConversion {
                error: e.to_string(),
            }
        })?);
    }

    // Dict
    if let Ok(dict) = any.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (k_any, v_any) in dict.iter() {
            // Key -> string per your rules
            let key = if k_any.is_none() {
                "null".to_string()
            } else if let Ok(b) = k_any.extract::<bool>() {
                if b {
                    "true".into()
                } else {
                    "false".into()
                }
            } else if let Ok(s) = k_any.str() {
                s.to_string()
            } else {
                return Err(PyCanonicalJSONError::DictKeyNotSerializable {
                    typename: type_name(&k_any)?,
                });
            };
            let v_json = to_json(py, &v_any.unbind())?;
            map.insert(key, v_json);
        }
        return Ok(JsonValue::Object(map));
    }

    // List
    if let Ok(lst) = any.downcast::<PyList>() {
        let mut out = Vec::with_capacity(lst.len());
        for item in lst.iter() {
            out.push(to_json(py, &item.unbind())?);
        }
        return Ok(JsonValue::Array(out));
    }

    // Tuple
    if let Ok(tup) = any.downcast::<PyTuple>() {
        let mut out = Vec::with_capacity(tup.len());
        for item in tup.iter() {
            out.push(to_json(py, &item.unbind())?);
        }
        return Ok(JsonValue::Array(out));
    }

    // Float (reject NaN/Inf)
    if let Ok(f) = any.downcast::<PyFloat>() {
        let val = f.value();
        match serde_json::Number::from_f64(val) {
            Some(n) => return Ok(JsonValue::Number(n)),
            None => {
                return Err(PyCanonicalJSONError::InvalidFloat {
                    typename: type_name(&f.as_any())?,
                })
            }
        }
    }

    // Fallback: unsupported
    Err(PyCanonicalJSONError::InvalidCast {
        typename: type_name(&any)?,
    })
}
