all: wheels

wheels:
	docker pull ghcr.io/pyo3/maturin
	docker run --rm -v $(shell pwd):/io ghcr.io/pyo3/maturin build --release -i python3.7 -i python3.8 -i python3.9 -i python3.10 -i python3.11
	docker run --rm -v $(shell pwd):/io ghcr.io/pyo3/maturin build --release --compatibility musllinux_1_1 -i python3.7 -i python3.8 -i python3.9 -i python3.10 -i python3.11

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

notebook: env
	. env/bin/activate && ipython kernel install --user --name=venv && jupyter notebook


