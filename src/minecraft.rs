use k8s_openapi::api::apps::v1::{StatefulSet, StatefulSetSpec};
use k8s_openapi::api::core::v1::{
    Container, ContainerPort, EnvVar, PersistentVolumeClaim, PersistentVolumeClaimSpec, PodSpec,
    PodTemplateSpec, Service, ServicePort, ServiceSpec, VolumeMount, VolumeResourceRequirements,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::api::{Api, DeleteParams, Patch, PatchParams};
use kube::core::ObjectMeta;
use kube::{CustomResource, CustomResourceExt};
use kube::{Resource, ResourceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::controller::Context;
use crate::Result;

static CONTROLLER_NAME: &str = "minecraft-operator";

#[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
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
    storage: MinecraftStorage,
    enable_node_port: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    /// motd is world name
    motd: String,
    /// gamemode (0-3)
    /// 0 Survival
    /// 1 Creative
    /// 2 Adventure
    /// 3 Spectator
    game_mode: GameMode,
    env: Option<Vec<EnvVar>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub enum GameMode {
    Survival = 0,
    Crative = 1,
    Adventure = 2,
    Spectator = 3,
}

/// Data Storage for Minecraft Server
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct MinecraftStorage {
    /// storage size should be quantitity
    size: String,
    /// data volume mount target path
    mount_path: String,
    /// CSI storage class name
    storage_class_name: String,
}

impl Minecraft {
    fn name(&self) -> String {
        format!("minecraft-{}", self.name_any())
    }

    pub async fn sync(&self, ctx: Arc<Context>) -> Result<(), kube::Error> {
        let name = self.name();
        let ns = self.namespace().unwrap();
        let ps = PatchParams::apply(CONTROLLER_NAME);

        // StatefulSet
        let statefulset = self.build_statefulset();
        let statefulset_api: Api<StatefulSet> = Api::namespaced(ctx.client.clone(), &ns);
        let patch = Patch::Apply(&statefulset);
        statefulset_api.patch(&name, &ps, &patch).await?;

        // Service
        let service = self.build_service();
        let service_api: Api<Service> = Api::namespaced(ctx.client.clone(), &ns);
        let patch = Patch::Apply(&service);
        service_api.patch(&name, &ps, &patch).await?;

        Ok(())
    }

    pub async fn delete_statefulset(&self, ctx: Arc<Context>) -> Result<(), kube::Error> {
        let name = self.name();
        let ns = self.namespace().unwrap();

        let statefulset_api: Api<StatefulSet> = Api::namespaced(ctx.client.clone(), &ns);

        let dp = DeleteParams::default();
        statefulset_api.delete(&name, &dp).await?;

        Ok(())
    }

    pub async fn delete_service(&self, ctx: Arc<Context>) -> Result<(), kube::Error> {
        let name = self.name();
        let ns = self.namespace().unwrap();

        let service_api: Api<Service> = Api::namespaced(ctx.client.clone(), &ns);
        let dp = DeleteParams::default();
        service_api.delete(&name, &dp).await?;
        Ok(())
    }

    pub fn default_labels(&self) -> BTreeMap<String, String> {
        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), "minecraft".to_string());
        labels
    }

    pub fn default_ports(&self) -> Vec<ContainerPort> {
        vec![
            ContainerPort {
                name: Some("minecraft-tcp".to_string()),
                container_port: 25565,
                protocol: Some("TCP".to_string()),
                host_ip: None,
                host_port: None,
            },
            ContainerPort {
                name: Some("minecraft-udp".to_string()),
                container_port: 25565,
                protocol: Some("UDP".to_string()),
                host_ip: None,
                host_port: None,
            },
            ContainerPort {
                name: Some("minecraft-rcon".to_string()),
                container_port: 25575,
                protocol: Some("UDP".to_string()),
                host_ip: None,
                host_port: None,
            },
        ]
    }

    pub fn default_service_ports(&self) -> Vec<ServicePort> {
        vec![
            ServicePort {
                name: Some("minecraft-tcp".to_string()),
                port: 25565,
                protocol: Some("TCP".to_string()),
                target_port: Some(IntOrString::Int(25565)),
                node_port: if self.spec.enable_node_port{
                    Some(30565)
                } else {
                    None
                },
                ..Default::default()
            },
            ServicePort {
                name: Some("minecraft-udp".to_string()),
                port: 25565,
                protocol: Some("UDP".to_string()),
                target_port: Some(IntOrString::Int(25565)),
                node_port: if self.spec.enable_node_port{
                    Some(30565)
                } else {
                    None
                },
                ..Default::default()
            },
            ServicePort {
                name: Some("minecraft-rcon".to_string()),
                port: 25575,
                protocol: Some("UDP".to_string()),
                target_port: Some(IntOrString::Int(25575)),
                node_port: if self.spec.enable_node_port{
                    Some(30575)
                } else {
                    None
                },
                ..Default::default()
            },
        ]
    }

    pub fn volume_claim(&self, name: &str) -> Vec<PersistentVolumeClaim> {
        let mut requests = BTreeMap::new();
        requests.insert(
            "storage".to_string(),
            Quantity(self.spec.storage.size.clone()),
        );

        let storage_resources = VolumeResourceRequirements {
            requests: Some(requests),
            ..Default::default()
        };

        let pvc = PersistentVolumeClaim {
            metadata: ObjectMeta {
                name: Some(format!("{name}-data")),
                labels: Some(self.default_labels()),
                ..Default::default()
            },
            spec: Some(PersistentVolumeClaimSpec {
                access_modes: Some(vec!["ReadWriteOnce".to_string()]),
                storage_class_name: Some(self.spec.storage.storage_class_name.clone()),
                resources: Some(storage_resources),
                ..Default::default()
            }),
            status: None,
        };

        vec![pvc]
    }

    pub fn default_metadata(&self, name: &str) -> ObjectMeta {
        let owner_ref = self.controller_owner_ref(&()).unwrap();
        ObjectMeta {
            name: Some(name.to_string()),
            labels: Some(self.default_labels()),
            owner_references: Some(vec![owner_ref]),
            ..Default::default()
        }
    }

    pub fn build_statefulset(&self) -> StatefulSet {
        let name = self.name();
        let labels = self.default_labels();

        let pod_spec = PodSpec {
            containers: vec![Container {
                image: Some(self.spec.image.clone()),
                env: self.spec.server.env.clone(),
                ports: Some(self.default_ports()),
                name: "minecraft-server".to_string(),
                volume_mounts: Some(vec![VolumeMount {
                    mount_path: self.spec.storage.mount_path.clone(),
                    name: format!("{name}-data"),
                    read_only: Some(false),
                    ..Default::default()
                }]),

                ..Default::default()
            }],
            ..Default::default()
        };

        let statefulset_spec = StatefulSetSpec {
            replicas: Some(1),
            service_name: name.clone(),
            selector: LabelSelector {
                match_expressions: None,
                match_labels: Some(labels),
            },
            template: PodTemplateSpec {
                metadata: Some(self.default_metadata(&name)),
                spec: Some(pod_spec),
            },
            volume_claim_templates: Some(self.volume_claim(&name)),
            persistent_volume_claim_retention_policy: None,
            min_ready_seconds: None,
            update_strategy: None,
            pod_management_policy: None,
            revision_history_limit: None,
            ordinals: None,
        };

        StatefulSet {
            metadata: self.default_metadata(&name),
            spec: Some(statefulset_spec),
            ..Default::default()
        }
    }

    pub fn build_service(&self) -> Service {
        let name = self.name();
        let type_ = if self.spec.enable_node_port{
            "NodePort"
        } else {
            "ClusterIP"
        };

        Service {
            metadata: self.default_metadata(&name),
            spec: Some(ServiceSpec {
                selector: Some(self.default_labels()),
                ports: Some(self.default_service_ports()),
                type_: Some(type_.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}

pub fn generate_crds() -> anyhow::Result<()> {
    let crd = serde_yaml::to_string(&Minecraft::crd())?;
    println!("{crd}");
    Ok(())
}
