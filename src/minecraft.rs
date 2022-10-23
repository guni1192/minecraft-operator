use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{Container, ContainerPort, EnvVar, PodSpec, PodTemplateSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use kube::api::{Api, DeleteParams, Patch, PatchParams};
use kube::core::ObjectMeta;
use kube::ResourceExt;
use kube::{CustomResource, CustomResourceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::controller::Context;
use crate::Result;

static CONTROLLER_NAME: &str = "minecraft-operator";

#[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[kube(
    group = "guni.dev",
    version = "v1",
    kind = "Minecraft",
    shortname = "mc",
    namespaced,
    derive = "PartialEq"
)]
pub struct MinecraftSpec {
    image: String,
    server: Server,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct Server {
    /// motd is world name
    motd: String,
    /// gamemode (0-3)
    /// 0 Survival
    /// 1 Creative
    /// 2 Adventure
    /// 3 Spectator
    gamemode: Gamemode,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub enum Gamemode {
    Survival = 0,
    Crative = 1,
    Adventure = 2,
    Spectator = 3,
}

impl Minecraft {
    pub async fn sync(&self, ctx: Arc<Context>) -> Result<(), kube::Error> {
        let name = self.name_any();
        let ns = self.namespace().unwrap();
        let dep = self.make_deployment()?;

        let deployment_api: Api<Deployment> = Api::namespaced(ctx.client.clone(), &ns);

        let ps = PatchParams::apply(CONTROLLER_NAME);
        let patch = Patch::Apply(&dep);
        deployment_api.patch(&name, &ps, &patch).await?;

        Ok(())
    }

    pub async fn delete_deployment(&self, ctx: Arc<Context>) -> Result<(), kube::Error> {
        let name = self.name_any();
        let ns = self.namespace().unwrap();

        let deployment_api: Api<Deployment> = Api::namespaced(ctx.client.clone(), &ns);

        let dp = DeleteParams::default();
        deployment_api.delete(&name, &dp).await?;

        Ok(())
    }

    pub fn labels(&self) -> BTreeMap<String, String> {
        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), "minecraft".to_string());
        labels
    }

    pub fn make_deployment(&self) -> Result<Deployment, kube::Error> {
        let name = self.name_any();
        let labels = self.labels();

        let meta = ObjectMeta {
            name: Some(name.to_string()),
            labels: Some(labels.clone()),
            ..Default::default()
        };

        let pod_spec = PodSpec {
            containers: vec![Container {
                image: Some(self.spec.image.clone()),
                env: Some(vec![EnvVar {
                    name: "EULA".to_string(),
                    value: Some("TRUE".to_string()),
                    value_from: None,
                }]),
                ports: Some(vec![
                    ContainerPort {
                        name: Some("minecraft".to_string()),
                        container_port: 25565,
                        protocol: Some("TCP".to_string()),
                        host_ip: None,
                        host_port: Some(25565),
                    },
                    ContainerPort {
                        name: Some("minecraft-udp".to_string()),
                        container_port: 25565,
                        protocol: Some("UDP".to_string()),
                        host_ip: None,
                        host_port: Some(25565),
                    },
                ]),
                name: "minecraft-server".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let deployment_spec = DeploymentSpec {
            replicas: Some(1),
            selector: LabelSelector {
                match_expressions: None,
                match_labels: Some(labels.clone()),
            },
            template: PodTemplateSpec {
                metadata: Some(meta.clone()),
                spec: Some(pod_spec),
            },
            ..Default::default()
        };

        let deployment = Deployment {
            metadata: meta,
            spec: Some(deployment_spec),
            ..Default::default()
        };

        Ok(deployment)
    }
}

pub fn generate_crds() -> anyhow::Result<()> {
    let crd = serde_yaml::to_string(&Minecraft::crd())?;
    println!("{}", crd);
    Ok(())
}
