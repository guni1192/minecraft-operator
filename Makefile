IMAGE=guni1192/minecraft-operator:static

build:
	DOCKER_BUILDKIT=1 docker image build -t $(IMAGE) .

run:
	docker run --rm --net=host \
		--mount type=bind,src=$(HOME)/.kube,dst=/root/.kube \
		-e KUBECONFIG=/root/.kube/config \
		$(IMAGE)
