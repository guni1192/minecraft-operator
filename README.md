# Minecraft Operator

```console
kind create cluster
docker build -t guni1192/minecraft-operator .
docker run --net=host --mount type=bind,src=$HOME/.kube,dst=/root/.kube guni1192/minecraft-operator
```
