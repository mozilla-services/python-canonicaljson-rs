use canonical_json::ser::{to_string, CanonicalJSONError};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::{
    types::{PyAny, PyDict, PyFloat, PyList, PyTuple},
    wrap_pyfunction,
};

pub enum PyCanonicalJSONError {
    InvalidConversion { error: String },
    PyErr { error: String },
    DictKeyNotSerializable { typename: String },
    InvalidFloat { value: PyObject },
    InvalidCast { typename: String },
}

impl From<CanonicalJSONError> for PyCanonicalJSONError {
    fn from(error: CanonicalJSONError) -> PyCanonicalJSONError {
        PyCanonicalJSONError::InvalidConversion {
            error: format!("{:?}", error),
        }
    }
}

impl From<pyo3::PyErr> for PyCanonicalJSONError {
    fn from(error: pyo3::PyErr) -> PyCanonicalJSONError {
        PyCanonicalJSONError::PyErr {
            error: format!("{:?}", error),
        }
    }
}

impl From<PyCanonicalJSONError> for pyo3::PyErr {
    fn from(e: PyCanonicalJSONError) -> pyo3::PyErr {
        match e {
            PyCanonicalJSONError::InvalidConversion { error } => {
                PyErr::new::<PyTypeError, _>(format!("Conversion error: {:?}", error))
            }
            PyCanonicalJSONError::PyErr { error } => {
                PyErr::new::<PyTypeError, _>(format!("Python Runtime exception: {}", error))
            }
            PyCanonicalJSONError::DictKeyNotSerializable { typename } => {
                PyErr::new::<PyTypeError, _>(format!(
                    "Dictionary key is not serializable: {}",
                    typename
                ))
            }
            PyCanonicalJSONError::InvalidFloat { value } => {
                PyErr::new::<PyTypeError, _>(format!("Invalid float: {:?}", value))
            }
            PyCanonicalJSONError::InvalidCast { typename } => {
                PyErr::new::<PyTypeError, _>(format!("Invalid type: {}", typename))
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
pub fn dump(py: Python, obj: PyObject, fp: PyObject) -> PyResult<PyObject> {
    let s = dumps(py, obj)?;
    let fp_ref: &PyAny = fp.extract(py)?;
    fp_ref.call_method1("write", (s,))?;

    Ok(pyo3::Python::None(py))
}

#[pyfunction]
pub fn dumps(py: Python, obj: PyObject) -> PyResult<PyObject> {
    let v = to_json(py, &obj)?;
    match to_string(&v) {
        Ok(s) => Ok(s.to_object(py)),
        Err(e) => Err(PyErr::new::<PyTypeError, _>(format!("{:?}", e))),
    }
}

fn to_json(py: Python, obj: &PyObject) -> Result<serde_json::Value, PyCanonicalJSONError> {
    macro_rules! return_cast {
        ($t:ty, $f:expr) => {
            if let Ok(val) = obj.downcast::<$t>(py) {
                return $f(val);
            }
        };
    }

    macro_rules! return_to_value {
        ($t:ty) => {
            if let Ok(val) = obj.extract::<$t>(py) {
                return serde_json::value::to_value(val).map_err(|error| {
                    PyCanonicalJSONError::InvalidConversion {
                        error: format!("{}", error),
                    }
                });
            }
        };
    }

    if obj.bind(py).eq(&py.None())? {
        return Ok(serde_json::Value::Null);
    }

    return_to_value!(String);
    return_to_value!(bool);
    return_to_value!(u64);
    return_to_value!(i64);

    return_cast!(PyDict, |x: &PyDict| {
        let mut map = serde_json::Map::new();
        for (key_obj, value) in x.iter() {
            let key = if key_obj.eq(py.None().bind(py))? {
                Ok("null".to_string())
            } else if let Ok(val) = key_obj.extract::<bool>() {
                Ok(if val {
                    "true".to_string()
                } else {
                    "false".to_string()
                })
            } else if let Ok(val) = key_obj.str() {
                Ok(val.to_string())
            } else {
                Err(PyCanonicalJSONError::DictKeyNotSerializable {
                    typename: key_obj
                        .to_object(py)
                        .bind(py)
                        .get_type()
                        .name()?
                        .to_string(),
                })
            };
            map.insert(key?, to_json(py, &value.to_object(py))?);
        }
        Ok(serde_json::Value::Object(map))
    });

    return_cast!(PyList, |x: &PyList| {
        let json_array: Result<Vec<_>, _> =
            x.iter().map(|x| to_json(py, &x.to_object(py))).collect(); // This turns the iterator into a Result<Vec<Value>, PyCanonicalJSONError>
        Ok(serde_json::Value::Array(json_array?))
    });

    return_cast!(PyTuple, |x: &PyTuple| {
        let json_array: Result<Vec<_>, _> =
            x.iter().map(|x| to_json(py, &x.to_object(py))).collect(); // This turns the iterator into a Result<Vec<Value>, PyCanonicalJSONError>
        Ok(serde_json::Value::Array(json_array?))
    });

    return_cast!(PyFloat, |x: &PyFloat| {
        match serde_json::Number::from_f64(x.value()) {
            Some(n) => Ok(serde_json::Value::Number(n)),
            None => Err(PyCanonicalJSONError::InvalidFloat {
                value: x.to_object(py),
            }),
        }
    });

    // At this point we can't cast it, set up the error object
    Err(PyCanonicalJSONError::InvalidCast {
        typename: obj.bind(py).get_type().name()?.to_string(),
    })
}
