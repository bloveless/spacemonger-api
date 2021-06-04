daemon_tag = 0.1.0-alpha.80
tor_tag = 0.1.0-alpha.4
api_tag = 0.1.0-alpha.19

publish-daemon:
	docker build --platform linux/arm64 -f docker/daemon/Dockerfile -t bloveless/spacemongerd:$(daemon_tag) .
	docker push bloveless/spacemongerd:$(daemon_tag)

publish-tor:
	docker build --platform linux/arm64 -f docker/tor/Dockerfile -t bloveless/tor:$(tor_tag) .
	docker push bloveless/tor:$(tor_tag)

deploy:
	kubectl apply -k ./k8s/

publish-api:
	docker build --platform linux/arm64 -f docker/api/Dockerfile -t bloveless/spacemonger-api:$(api_tag) .
	docker push bloveless/spacemonger-api:$(api_tag)

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
