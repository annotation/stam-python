all: wheels

wheels:
	docker pull ghcr.io/pyo3/maturin
	docker run --rm -v $(shell pwd):/io ghcr.io/pyo3/maturin build --release

build:
	cargo build --release

env:
	python -m venv env
	. env/bin/activate; \
	pip install .

clean:
	-rm -Rf env

local:
	cargo build --config 'patch.crates-io.stam.path="../stam-rust/"'
