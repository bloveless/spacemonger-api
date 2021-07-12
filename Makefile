include .env

daemon_tag = 0.2.0-alpha.17
api_tag = 0.2.0-alpha.2

.PHONY: publish-daemon
publish-daemon:
	docker build --platform linux/arm64 -f docker/daemon/Dockerfile -t bloveless/spacemongerd:$(daemon_tag) .
	docker push bloveless/spacemongerd:$(daemon_tag)

.PHONY: publish-api
publish-api:
	docker build --platform linux/arm64 -f docker/api/Dockerfile -t bloveless/spacemonger-api:$(api_tag) .
	docker push bloveless/spacemonger-api:$(api_tag)

.PHONY: deploy
deploy:
	kubectl apply -k ./k8s/

.PHONY: migration
migration:
	migrate create -ext sql -dir ./migrations $(name)

.PHONY: migrate
migrate:
	migrate -source file://migrations -database $(POSTGRES_URL) up

.PHONY: rollback
rollback:
	migrate -source file://migrations -database $(POSTGRES_URL) down 1

.PHONY: psql
psql:
	docker-compose exec postgres psql -U spacemonger

.PHONY: psql-test
psql-test:
	docker-compose exec postgres psql -U spacemonger_test

# watch-api:
# 	cargo watch -x 'run --package spacemonger-api --bin spacemonger-api'

# watch-daemon:
# 	cargo watch -x 'run --package spacemonger-daemon --bin spacemongerd'
