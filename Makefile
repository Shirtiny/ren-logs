.DEFAULT_GOAL := build
.ONESHELL:
SHELL := /bin/bash

init:
	@echo "Initialising the project"
	# @sudo chmod -R 777 .scripts
	@pnpm i
	# @npx msw init -y
	# 
	@npx update-browserslist-db@latest
	@pnpm node ./.scripts/init.cjs

build_arch: test
	@echo "✅"

clean:
	@echo "🛁 Cleaning..."
	@pnpm clean

test:
	@echo "Testing..."
	@./.scripts/test.sh

build:init
	@echo "👩‍🏭 Building..."
	pnpm build
	pnpm site
	
start:clean
	pnpm desktop




