publish:
	docker buildx build --platform linux/arm64,linux/amd64 -t bloveless/spacetraders:0.1.0-alpha.7 --push .

migration:
	DATABASE_URL=postgresql://spacetraders:2djlsUYwcF0YzSgvTZPc9BCWff@localhost:5433 sqlx migrate add $(name)

migrate:
	DATABASE_URL=postgresql://spacetraders:2djlsUYwcF0YzSgvTZPc9BCWff@localhost:5433 sqlx migrate run
