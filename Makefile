publish-daemon:
	docker buildx build --platform linux/arm64 -f docker/daemon/Dockerfile -t bloveless/spacetradersd:0.1.0-alpha.1 --push .

publish-api:
	docker buildx build --platform linux/arm64 -f docker/api/Dockerfile -t bloveless/spacetraders-api:0.1.0-alpha.2 --push .

migration-daemon:
	cd daemon; DATABASE_URL=postgresql://spacetraders:2djlsUYwcF0YzSgvTZPc9BCWff@localhost:5433 sqlx migrate add $(name)

migration-api:
	cd api; DATABASE_URL=postgresql://spacetraders:2djlsUYwcF0YzSgvTZPc9BCWff@localhost:5433 sqlx migrate add $(name)

migrate-daemon:
	cd daemon; DATABASE_URL=postgresql://spacetraders:2djlsUYwcF0YzSgvTZPc9BCWff@localhost:5433 sqlx migrate run

migrate-api:
	cd api; DATABASE_URL=postgresql://spacetraders:2djlsUYwcF0YzSgvTZPc9BCWff@localhost:5433 sqlx migrate run

watch-api:
	cargo watch -x 'run --package spacetraders-api --bin spacetraders-api'

watch-daemon:
	cargo watch -x 'run --package spacetraders-daemon --bin spacetradersd'
