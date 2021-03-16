publish:
	docker buildx build --platform linux/arm64 -t bloveless/spacetraders:0.1.0-alpha.4 --push .
