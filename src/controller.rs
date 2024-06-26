use std::sync::Arc;

use crate::minecraft::Minecraft;
use crate::{Error, Result};
use chrono::{DateTime, Utc};
use futures::{future::BoxFuture, FutureExt, StreamExt};
use kube::runtime::watcher::Config;
use kube::{
    api::{Api, ListParams, ResourceExt},
    client::Client,
    runtime::{
        controller::{Action, Controller},
        events::{Event, EventType, Recorder, Reporter},
        finalizer::{finalizer, Event as FinalizerEvent},
    },
    Resource,
};

use prometheus::{default_registry, proto::MetricFamily};
use serde::Serialize;
use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing::{info, instrument, warn};

static MINECRAFT_FINALIZER: &str = "minecraft.guni.dev";

#[derive(Clone)]
pub struct Context {
    /// Kubernetes client
    pub client: Client,
    /// Diagnostics read by the web server
    pub diagnostics: Arc<RwLock<Diagnostics>>,
    /// Prometheus metrics
    pub metrics: Metrics,
}

impl Minecraft {
    async fn reconcile(&self, ctx: Arc<Context>) -> Result<Action, kube::Error> {
        ctx.diagnostics.write().await.last_event = Utc::now();
        self.sync(ctx).await?;

        Ok(Action::requeue(Duration::from_secs(5 * 60)))
    }

    async fn cleanup(&self, ctx: Arc<Context>) -> Result<Action, kube::Error> {
        let client = ctx.client.clone();
        ctx.diagnostics.write().await.last_event = Utc::now();
        let reporter = ctx.diagnostics.read().await.reporter.clone();
        let recorder = Recorder::new(client.clone(), reporter, self.object_ref(&()));

        self.delete_statefulset(ctx.clone()).await?;
        self.delete_service(ctx).await?;

        recorder
            .publish(Event {
                type_: EventType::Normal,
                reason: "DeleteMinecraft".into(),
                note: Some(format!("Delete `{}`", self.name_any())),
                action: "Reconciling".into(),
                secondary: None,
            })
            .await?;

        Ok(Action::await_change())
    }
}

/// Data owned by the Manager
#[derive(Clone)]
pub struct Manager {
    /// Diagnostics populated by the reconciler
    diagnostics: Arc<RwLock<Diagnostics>>,
}

impl Manager {
    pub async fn new() -> (Self, BoxFuture<'static, ()>) {
        let client = Client::try_default().await.expect("create client");
        let metrics = Metrics::new();
        let diagnostics = Arc::new(RwLock::new(Diagnostics::new()));
        let context = Arc::new(Context {
            client: client.clone(),
            metrics: metrics.clone(),
            diagnostics: diagnostics.clone(),
        });

        let minecraft_api = Api::<Minecraft>::all(client);
        let _r = minecraft_api
            .list(&ListParams::default().limit(1))
            .await
            .expect(
                "is the crd installed? please run: minecraft-operator crd-gen | kubectl apply -f -",
            );

        let controller = Controller::new(minecraft_api, Config::default())
            .run(reconcile, error_policy, context)
            .filter_map(|x| async move { std::result::Result::ok(x) })
            .for_each(|_| futures::future::ready(()))
            .boxed();

        (Self { diagnostics }, controller)
    }

    pub fn metrics(&self) -> Vec<MetricFamily> {
        default_registry().gather()
    }

    pub async fn diagnostics(&self) -> Diagnostics {
        self.diagnostics.read().await.clone()
    }
}

/// Prometheus Metrics to be exposed on /metrics
#[derive(Clone)]
pub struct Metrics {}

impl Metrics {
    fn new() -> Self {
        Metrics {}
    }
}

#[derive(Clone, Serialize)]
pub struct Diagnostics {
    #[serde(deserialize_with = "from_ts")]
    pub last_event: DateTime<Utc>,
    #[serde(skip)]
    pub reporter: Reporter,
}

impl Diagnostics {
    fn new() -> Self {
        Self {
            last_event: Utc::now(),
            reporter: "minecraft-operator".into(),
        }
    }
}

fn error_policy(mc: Arc<Minecraft>, error: &Error, _ctx: Arc<Context>) -> Action {
    let name = mc.name_any();
    warn!(
        "reconcile failed minecraft.guni.dev resource: name: {}, Error: {:?}",
        name, error
    );
    Action::requeue(Duration::from_secs(5 * 60))
}

#[instrument(skip(ctx, mc))]
async fn reconcile(mc: Arc<Minecraft>, ctx: Arc<Context>) -> Result<Action> {
    let client = ctx.client.clone();
    let name = mc.name_any();
    let ns = mc.namespace().unwrap();
    let minecraft_api: Api<Minecraft> = Api::namespaced(client, &ns);

    let action = finalizer(&minecraft_api, MINECRAFT_FINALIZER, mc, |event| async {
        match event {
            FinalizerEvent::Apply(mc) => mc.reconcile(ctx.clone()).await,
            FinalizerEvent::Cleanup(mc) => mc.cleanup(ctx.clone()).await,
        }
    })
    .await
    .map_err(Error::FinalizerError);

    info!("Reconciled Minecraft \"{}\" in \"{}\" ", name, ns);
    action
}
