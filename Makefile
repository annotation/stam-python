wheels:
	docker pull konstin2/maturin
	docker run --rm -v $(shell pwd):/io konstin2/maturin build --release
