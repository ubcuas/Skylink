## Util ##
list:
	@$(MAKE) -pRrq -f $(lastword $(MAKEFILE_LIST)) : 2>/dev/null | awk -v RS= -F: '/^# File/,/^# Finished Make data base/ {if ($$1 !~ "^[#.]") {print $$1}}' | sort | egrep -v -e '^[^[:alnum:]]' -e '^$@$$'

## Build ##
build:
	cargo build

build-release:
	cargo build --release

## Test ##
test:
	cargo test

test-release:
	cargo test --release

## Cleanup ##
clean:
	cargo clean

## Install ##
install:
	cargo install

## Docker ##
docker:
	docker build . -t ubcuas/skylink:latest

docker-publish: docker
	docker push ubcuas/skylink:latest

## CI ##
ci-test:
	docker build . --target build -t ubcuas/skylink:test
	docker run ubcuas/skylink:test cargo test --release
