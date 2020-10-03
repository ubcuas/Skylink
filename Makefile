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
docker-multiarch-deps:
	DOCKER_CLI_EXPERIMENTAL=enabled DOCKER_BUILDKIT=enabled docker run --rm --privileged multiarch/qemu-user-static --reset -p yes
	DOCKER_CLI_EXPERIMENTAL=enabled DOCKER_BUILDKIT=enabled docker buildx create --name mubuilder | echo "ok"
	DOCKER_CLI_EXPERIMENTAL=enabled DOCKER_BUILDKIT=enabled docker buildx use mubuilder
	DOCKER_CLI_EXPERIMENTAL=enabled DOCKER_BUILDKIT=enabled docker buildx inspect --bootstrap

docker:
	docker build . --pull=true --tag ubcuas/skylink:latest

docker-publish: docker
	docker push ubcuas/skylink:latest

docker-multiarch: docker-multiarch-deps
	DOCKER_CLI_EXPERIMENTAL=enabled \
	DOCKER_BUILDKIT=enabled \
	docker buildx build . --pull=true -t ubcuas/skylink:latest --platform "linux/amd64,linux/arm64"

docker-multiarch-publish: docker-multiarch-deps
	DOCKER_CLI_EXPERIMENTAL=enabled \
	DOCKER_BUILDKIT=enabled \
	docker buildx build . --pull=true -t ubcuas/skylink:latest --push --platform "linux/amd64,linux/arm64"

## CI ##
ci-test:
	docker build . --pull=true --target build -t ubcuas/skylink:test
	docker run ubcuas/skylink:test cargo test --release
