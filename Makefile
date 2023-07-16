IMAGE=guni1192/minecraft-operator:dev
CLUSTER_NAME=k8s-minecraft

build:
	docker buildx build -t $(IMAGE) .
	kind load docker-image --name $(CLUSTER_NAME) $(IMAGE)

run:
	docker run --rm --net=host \
		--mount type=bind,src=$(HOME)/.kube,dst=/root/.kube \
		-e KUBECONFIG=/root/.kube/config \
		$(IMAGE)

install: build
	docker container run --rm guni1192/minecraft-operator:dev crd-gen | kubectl apply -f -

deploy: build
	kustomize build config/base | kubectl apply --server-side -f -
	kubectl rollout -n minecraft-system restart deployment minecraft-operator

delete:
	kubectl delete -f config/minecraft-operator/

create-cluster:
	kind create cluster --name $(CLUSTER_NAME) --config kindconfig.yaml
	kubectl cluster-info --context kind-$(CLUSTER_NAME)

delete-cluster:
	kind delete cluster --name $(CLUSTER_NAME)

