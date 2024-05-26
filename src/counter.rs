use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use wmi::COMLibrary;
use wmi::WMIConnection;

use anyhow::Context as _;
use anyhow::Result;
use tokio::sync::Mutex;
use wmi::FilterValue;

#[derive(Debug)]
struct CounterInner {
    re: regex::Regex,
    savefile: PathBuf,
    count: u64,
}

#[derive(Debug)]
pub struct Counter {
    inner: Arc<Mutex<CounterInner>>,
}

impl Drop for Counter {
    fn drop(&mut self) {
        let inner = self.inner.blocking_lock();
        let count = inner.count;
        std::fs::write(&inner.savefile, count.to_string())
            .expect("failed to dump count to save file");
    }
}

impl Counter {
    pub fn new(window_title_re: &str, savefile: &str) -> Result<Self> {
        let re = regex::Regex::new(window_title_re).context("failed to ")?;
        Ok(Self {
            inner: Arc::new(Mutex::new(CounterInner {
                re,
                count: 0,
                savefile: savefile.into(),
            })),
        })
    }

    async fn add(&self, event: &NewProcessEvent) {
        let mut inner = self.inner.lock().await;
        if inner.re.is_match(&event.target_instance.name) {
            inner.count += 1;
        }
    }

    pub async fn get_count(&self) -> u64 {
        let inner = self.inner.lock().await;
        inner.count
    }

    pub async fn save(&self) -> Result<()> {
        let inner = self.inner.lock().await;
        let count = inner.count;
        std::fs::write(&inner.savefile, count.to_string())
            .expect("failed to dump count to save file");
        Ok(())
    }

    pub async fn load(&self) -> Result<()> {
        let mut inner = self.inner.lock().await;

        let file = std::fs::read(&inner.savefile).expect("failed to dump count to save file");
        let mut lines = file.lines();
        let line = lines.next_line().await?;

        if let Some(line) = line {
            let count: u64 = line.parse()?;
            inner.count = count;
        }

        Ok(())
    }

    pub async fn do_count(self: Arc<Self>, rchan: Arc<Mutex<UnboundedReceiver<NewProcessEvent>>>) {
        let rchan = rchan.clone();

        tokio::spawn({
            let counter = self.clone();
            async move {
                loop {
                    let Some(event) = rchan.lock().await.recv().await else {
                        tracing::error!("received null event through channel");
                        continue;
                    };
                    counter.add(&event).await;
                }
            }
        });
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename = "__InstanceCreationEvent")]
#[serde(rename_all = "PascalCase")]
pub struct NewProcessEvent {
    target_instance: Process,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_Process")]
#[serde(rename_all = "PascalCase")]
pub struct Process {
    name: String,
}

pub fn listen(schan: UnboundedSender<NewProcessEvent>) {
    let mut filters = HashMap::<String, FilterValue>::new();

    filters.insert(
        "TargetInstance".to_owned(),
        FilterValue::is_a::<Process>().unwrap(),
    );
    let wmi_con = WMIConnection::new(COMLibrary::new().unwrap()).unwrap();
    let iterator = wmi_con
        .filtered_notification::<NewProcessEvent>(&filters, Some(Duration::from_secs(1)))
        .unwrap();

    for event in iterator.filter_map(Result::ok) {
        schan
            .send(event)
            .unwrap_or_else(|e| tracing::error!(err = ?e, "Could not send event"))
    }
}
