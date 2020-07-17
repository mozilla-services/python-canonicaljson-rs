use pyo3::exceptions::TypeError as PyTypeError;
use pyo3::prelude::*;
use pyo3::{
    types::{PyAny, PyDict, PyFloat, PyList, PyTuple},
    wrap_pyfunction,
};

pub enum CanonicalJSONError {
    InvalidConversion { error: String },
    PyErr { error: String },
    DictKeyNotSerializable { typename: String },
    InvalidFloat { value: PyObject },
    InvalidCast { typename: String },
}

impl From<serde_json::Error> for CanonicalJSONError {
    fn from(error: serde_json::Error) -> CanonicalJSONError {
        CanonicalJSONError::InvalidConversion {
            error: format!("{:?}", error),
        }
    }
}

impl From<pyo3::PyErr> for CanonicalJSONError {
    fn from(error: pyo3::PyErr) -> CanonicalJSONError {
        CanonicalJSONError::PyErr {
            error: format!("{:?}", error),
        }
    }
}

impl From<CanonicalJSONError> for pyo3::PyErr {
    fn from(e: CanonicalJSONError) -> pyo3::PyErr {
        match e {
            CanonicalJSONError::InvalidConversion { error } => {
                PyErr::new::<PyTypeError, _>(format!("Conversion error: {}", error))
            }
            CanonicalJSONError::PyErr { error } => {
                PyErr::new::<PyTypeError, _>(format!("Python Runtime exception: {}", error))
            }
            CanonicalJSONError::DictKeyNotSerializable { typename } => {
                PyErr::new::<PyTypeError, _>(format!(
                    "Dictionary key is not serializable: {}",
                    typename
                ))
            }
            CanonicalJSONError::InvalidFloat { value } => {
                PyErr::new::<PyTypeError, _>(format!("Invalid float: {:?}", value))
            }
            CanonicalJSONError::InvalidCast { typename } => {
                PyErr::new::<PyTypeError, _>(format!("Invalid type: {}", typename))
            }
        }
    }
}

/// A canonical JSON serializer written in Rust
#[pymodule]
fn canonicaljson(_py: Python, m: &PyModule) -> PyResult<()> {
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
    match serde_json::to_string(&v) {
        Ok(s) => Ok(s.to_object(py)),
        Err(e) => Err(PyErr::new::<PyTypeError, _>(format!("{}", e))),
    }
}

fn to_json(py: Python, obj: &PyObject) -> Result<serde_json::Value, CanonicalJSONError> {
    macro_rules! return_cast {
        ($t:ty, $f:expr) => {
            if let Ok(val) = obj.cast_as::<$t>(py) {
                return $f(val);
            }
        };
    }

    macro_rules! return_to_value {
        ($t:ty) => {
            if let Ok(val) = obj.extract::<$t>(py) {
                return serde_json::value::to_value(val).map_err(|error| {
                    CanonicalJSONError::InvalidConversion {
                        error: format!("{}", error),
                    }
                });
            }
        };
    }

    if obj == &py.None() {
        return Ok(serde_json::Value::Null);
    }

    return_to_value!(String);
    return_to_value!(bool);
    return_to_value!(u64);
    return_to_value!(i64);

    return_cast!(PyDict, |x: &PyDict| {
        let mut map = serde_json::Map::new();
        for (key_obj, value) in x.iter() {
            let key = if key_obj == py.None().as_ref(py) {
                Ok("null".to_string())
            } else if let Ok(val) = key_obj.extract::<bool>() {
                Ok(if val {
                    "true".to_string()
                } else {
                    "false".to_string()
                })
            } else if let Ok(val) = key_obj.str() {
                Ok(val.to_string()?.into_owned())
            } else {
                Err(CanonicalJSONError::DictKeyNotSerializable {
                    typename: key_obj
                        .to_object(py)
                        .as_ref(py)
                        .get_type()
                        .name()
                        .into_owned(),
                })
            };
            map.insert(key?, to_json(py, &value.to_object(py))?);
        }
        Ok(serde_json::Value::Object(map))
    });

    return_cast!(PyList, |x: &PyList| Ok(serde_json::Value::Array(r#try!(x
        .iter()
        .map(|x| to_json(py, &x.to_object(py)))
        .collect()))));

    return_cast!(PyTuple, |x: &PyTuple| Ok(serde_json::Value::Array(r#try!(
        x.iter().map(|x| to_json(py, &x.to_object(py))).collect()
    ))));

    return_cast!(PyFloat, |x: &PyFloat| {
        match serde_json::Number::from_f64(x.value()) {
            Some(n) => Ok(serde_json::Value::Number(n)),
            None => Err(CanonicalJSONError::InvalidFloat {
                value: x.to_object(py),
            }),
        }
    });

    // At this point we can't cast it, set up the error object
    Err(CanonicalJSONError::InvalidCast {
        typename: obj.as_ref(py).get_type().name().into_owned(),
    })
}
