IMAGE=guni1192/minecraft-operator:dev
CLUSTER_NAME=k8s-minecraft
KIND_CONFIG=kind-config.yaml

build:
	DOCKER_BUILDKIT=1 docker image build -t $(IMAGE) .
	kind load docker-image --name $(CLUSTER_NAME) $(IMAGE)

run:
	docker run --rm --net=host \
		--mount type=bind,src=$(HOME)/.kube,dst=/root/.kube \
		-e KUBECONFIG=/root/.kube/config \
		$(IMAGE)

install:
	cargo run -- crd-gen | kubectl apply -f -

install-ingress:
	kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/kind/deploy.yaml
	kubectl wait --namespace ingress-nginx \
		--for=condition=ready pod \
		--selector=app.kubernetes.io/component=controller \
		--timeout=90s

deploy:
	kustomize build config/base | kubectl apply -f -

delete:
	kubectl delete -f config/minecraft-operator/

create-cluster:
	kind create cluster --name $(CLUSTER_NAME) --config $(KIND_CONFIG)
	kubectl cluster-info --context kind-$(CLUSTER_NAME)

delete-cluster:
	kind delete cluster --name $(CLUSTER_NAME)
