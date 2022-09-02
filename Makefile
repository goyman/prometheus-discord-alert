.PHONY: build

build:
	cargo build

.PHONY: release

release:
	cargo build --release

.PHONY: run

run:
	cargo run

.PHONY: clean

clean:
	rm -fr target
