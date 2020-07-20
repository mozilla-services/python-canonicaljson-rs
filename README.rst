canonicaljson-rs
################

Python package leveraging our Canonical JSON implementation in Rust.

In order to validate content signatures of our data, Canonical JSON gives us a predictable JSON serialization.
And Rust allows us to reuse the same implementation between our server in Python (this package) and our diverse clients (Rust, Android/iOS, JavaScript).

Usage
=====

.. code-block ::

    pip install canonicaljson-rs

.. code-block :: python

    >>> import canonicaljson
    >>>
    >>> canonicaljson.dumps({"foo": 42})
    '{"foo":42}'


* ``canonicaljson.dumps(obj: Any) -> str``
* ``canonicaljson.dump(obj: Any, stream: IO) -> str``


See Also
========

* https://github.com/Kinto/kinto-signer/blob/6.1.0/kinto_signer/canonicaljson.py
* https://searchfox.org/mozilla-central/rev/b2395478c/toolkit/modules/CanonicalJSON.jsm
* https://github.com/matrix-org/python-canonicaljson
* The code to build a ``serde_json::Value`` from a ``pyo3::PyObject`` was greatly inspired by Matthias Endler's `hyperjson <https://github.com/mre/hyperjson/>`_

Development
===========

We rely on a specific Python builder that automates everything around Rust bindings.

.. code-block ::

    pip install maturin

In order to install the package in the current environment:

.. code-block ::

    maturin develop

Build and Release:

.. code-block ::

    vim Cargo.toml
    git ci -am "Bump version"
    git tag -a v1.2.3

Update version in ``Cargo.toml`` and:

.. code-block ::

    maturin build
    maturin publish

License
=======

* Mozilla Public License 2.0
