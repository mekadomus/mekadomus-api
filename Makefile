image:
	@mkdir -p build/target
	@mkdir -p build/registry
	@docker build -f dockerfiles/dev -t mekadomus-api-image .
.PHONY: image

image-prod: build
	@docker run --rm \
		-v $(PWD)/assets/:/api/assets \
		-v $(PWD)/Cargo.lock:/api/Cargo.lock \
		-v $(PWD)/Cargo.toml:/api/Cargo.toml \
		-v $(PWD)/.env:/api/.env \
		-v $(PWD)/.env.sample:/api/.env.sample \
		-v $(PWD)/build/target:/api/target \
		-v $(PWD)/src:/api/src \
		-w /api/ \
		mekadomus-api-image \
		cargo build --release
	@docker build -f dockerfiles/prod -t api-image-prod .
.PHONY: image-prod

ssh: clean
	@docker run --rm -it \
		-p 3000:3000 \
		-v $(PWD)/assets/:/api/assets \
		-v $(PWD)/Cargo.lock:/api/Cargo.lock \
		-v $(PWD)/Cargo.toml:/api/Cargo.toml \
		-v $(PWD)/.env:/api/.env \
		-v $(PWD)/.env.sample:/api/.env.sample \
		-v $(PWD)/Makefile:/api/Makefile \
		-v $(PWD)/README.md:/api/README.md \
		-v $(PWD)/build:/api/build \
		-v $(PWD)/dockerfiles:/api/dockerfiles \
		-v $(PWD)/src:/api/src \
		-v $(PWD)/tests:/api/tests \
		-v $(PWD)/dev-environments/vim/tmp:/root/.local/share/nvim \
		-w /api/ \
		--name mekadomus-api-dev \
		mekadomus-api-image \
		bash
.PHONY: ssh

ssh-prod: image-prod
	@docker run --rm -it -p 3000:3000 \
		--name mekadomus-api-prod-ssh \
		api-image-prod \
		bash
.PHONY: ssh-prod

build: image
	@docker run --rm -it -p 3000:3000 \
		-v $(PWD)/assets/:/api/assets \
		-v $(PWD)/Cargo.lock:/api/Cargo.lock \
		-v $(PWD)/Cargo.toml:/api/Cargo.toml \
		-v $(PWD)/.env:/api/.env \
		-v $(PWD)/.env.sample:/api/.env.sample \
		-v $(PWD)/build/registry:/usr/local/cargo/registry \
		-v $(PWD)/build/target:/api/target \
		-v $(PWD)/src:/api/src \
		-v $(PWD)/tests:/api/tests \
		-w /api/ \
		--name mekadomus-api-dev \
		mekadomus-api-image \
		sh -c "cargo build && cargo test --no-run"
.PHONY: build

start:
	@docker compose -f dockerfiles/docker-compose-dev.yaml down --remove-orphans
	@docker compose -f dockerfiles/docker-compose-dev.yaml up --build --abort-on-container-exit
.PHONY: start

start-prod: image-prod
	@docker run --rm -it -p 3000:3000 \
		--env-file .env \
		--name mekadomus-api-prod \
		api-image-prod \
		./mekadomus_api
.PHONY: start-prod

# Starts a container with a neovim development environment ready to use
vim: image
	@docker build -f dockerfiles/dev-vim -t mekadomus-api-vim-image .
	@docker run --rm -it \
		-v $(PWD)/assets/:/api/assets \
		-v $(PWD)/Cargo.lock:/api/Cargo.lock \
		-v $(PWD)/Cargo.toml:/api/Cargo.toml \
		-v $(PWD)/.env:/api/.env \
		-v $(PWD)/.env.sample:/api/.env.sample \
		-v $(PWD)/Makefile:/api/Makefile \
		-v $(PWD)/README.md:/api/README.md \
		-v $(PWD)/build:/api/build \
		-v $(PWD)/dockerfiles:/api/dockerfiles \
		-v $(PWD)/src:/api/src \
		-v $(PWD)/tests:/api/tests \
		-v $(PWD)/dev-environments/vim/tmp:/root/.local/share/nvim \
		-w /api/ \
		mekadomus-api-vim-image \
		sh -c "nvim"
.PHONY: vim

test:
	@docker compose -f dockerfiles/docker-compose-test.yaml down --remove-orphans
	@docker compose -f dockerfiles/docker-compose-test.yaml up --build \
		--abort-on-container-exit \
		--exit-code-from api
.PHONY: test

fix:
	@docker run --rm -e "RUSTFLAGS=-Dwarnings" -v $(PWD)/:/api/ -w /api/ mekadomus-api-image sh -c "cargo fmt && cargo fix"
.PHONY: fix

check:
	@docker run --rm -e "RUSTFLAGS=-Dwarnings" -v $(PWD)/:/api/ -w /api/ mekadomus-api-image sh -c "cargo fmt --check && cargo check"
.PHONY: check

verify: clean build image-prod check test
.PHONY: verify

clean:
	-@docker kill mekadomus-api-dev 2>/dev/null ||:
	-@docker rm mekadomus-api-dev 2>/dev/null ||:
	-@docker kill mekadomus-api-prod 2>/dev/null ||:
	-@docker rm mekadomus-api-prod 2>/dev/null ||:
.PHONY: clean
