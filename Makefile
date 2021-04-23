publish-daemon:
	docker buildx build --platform linux/arm64 -f docker/daemon/Dockerfile -t bloveless/spacemongerd:0.1.0-alpha.3 --push .

publish-api:
	docker buildx build --platform linux/arm64 -f docker/api/Dockerfile -t bloveless/spacemonger-api:0.1.0-alpha.3 --push .

migration-daemon:
	cd daemon; DATABASE_URL=postgresql://spacemonger:2djlsUYwcF0YzSgvTZPc9BCWff@localhost:5433 sqlx migrate add $(name)

migration-api:
	cd api; DATABASE_URL=postgresql://spacemonger:2djlsUYwcF0YzSgvTZPc9BCWff@localhost:5433 sqlx migrate add $(name)

migrate-daemon:
	cd daemon; DATABASE_URL=postgresql://spacemonger:2djlsUYwcF0YzSgvTZPc9BCWff@localhost:5433 sqlx migrate run

migrate-api:
	cd api; DATABASE_URL=postgresql://spacemonger:2djlsUYwcF0YzSgvTZPc9BCWff@localhost:5433 sqlx migrate run

watch-api:
	cargo watch -x 'run --package spacemonger-api --bin spacemonger-api'

watch-daemon:
	cargo watch -x 'run --package spacemonger-daemon --bin spacemongerd'
