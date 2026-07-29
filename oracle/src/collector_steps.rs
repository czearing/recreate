use crate::{
    browser::Browser,
    checkpoint,
    collector_browser::{advance, reload, resize, wait_interaction_ready},
    collector_transition,
    model::{Checkpoint, Scenario, Step, Viewport},
    replay, transition,
};
use std::collections::BTreeMap;

pub(crate) struct Run<'a> {
    pub(crate) browser: &'a mut Browser,
    pub(crate) viewport: Viewport,
    pub(crate) checkpoints: Vec<Checkpoint>,
    responsive_cache: BTreeMap<(u32, u32), Checkpoint>,
    pub(crate) baseline_digest: String,
    pub(crate) baseline_transition: serde_json::Value,
    baseline_url: String,
    baseline_graph_digest: String,
    pub(crate) transition_state: Option<serde_json::Value>,
    pub(crate) clean: bool,
}

impl<'a> Run<'a> {
    pub(crate) async fn new(browser: &'a mut Browser) -> anyhow::Result<Self> {
        let baseline = transition::capture(&mut browser.cdp).await?;
        let baseline_url = browser
            .cdp
            .evaluate("location.href")
            .await?
            .as_str()
            .unwrap_or_default()
            .to_owned();
        Ok(Self {
            browser,
            viewport: Viewport {
                width: 1280,
                height: 800,
            },
            checkpoints: Vec::new(),
            responsive_cache: BTreeMap::new(),
            baseline_digest: transition::reset_digest(&baseline)?,
            baseline_transition: baseline.clone(),
            baseline_url,
            baseline_graph_digest: transition::state_digest(&baseline)?,
            transition_state: Some(baseline),
            clean: true,
        })
    }

    pub(crate) fn finish(self) -> Vec<Checkpoint> {
        self.checkpoints
    }

    pub(crate) fn fail(&mut self, scenario: &Scenario, step: usize, error: &anyhow::Error) {
        self.checkpoints.push(checkpoint::failure(
            &scenario.id,
            step,
            self.viewport.clone(),
            &format!("{error:#}"),
        ));
    }

    pub(crate) async fn execute(
        &mut self,
        scenario: &Scenario,
        step: &Step,
        index: &mut usize,
    ) -> anyhow::Result<()> {
        match step {
            Step::Reset => self.reset(index).await?,
            Step::SetViewport { width, height } => {
                self.viewport = resize(self.browser, *width, *height).await?;
                self.transition_state = None;
                self.push_responsive(scenario, *index).await?;
                *index += 1;
            }
            Step::ResizePath { widths, height } => {
                for width in widths {
                    self.viewport = resize(self.browser, *width, *height).await?;
                    self.transition_state = None;
                    self.push_responsive(scenario, *index).await?;
                    *index += 1;
                }
            }
            Step::AdvanceTime { milliseconds } => {
                self.browser.disable_network_fixture().await?;
                advance(self.browser, *milliseconds).await?;
                self.transition_state = None;
                self.push(scenario, *index).await?;
                self.clean = false;
                *index += 1;
            }
            Step::Activate { anchor } => {
                let before = collector_transition::before(self).await?;
                let after = replay::activate(self.browser, anchor).await?;
                collector_transition::push(self, scenario, *index, &before, after).await?;
                *index += 1;
            }
            Step::Hover { anchor } => {
                let before = collector_transition::before(self).await?;
                let after = replay::hover(self.browser, anchor).await?;
                collector_transition::push(self, scenario, *index, &before, after).await?;
                *index += 1;
            }
            Step::PrepareActivate { anchor } => {
                let after = replay::activate(self.browser, anchor).await?;
                collector_transition::update(self, after)?;
                *index += 1;
            }
            Step::PrepareHover { anchor } => {
                let after = replay::hover(self.browser, anchor).await?;
                collector_transition::update(self, after)?;
                *index += 1;
            }
            Step::Key { key } => {
                let before = collector_transition::before(self).await?;
                let after = replay::key(self.browser, key).await?;
                collector_transition::push(self, scenario, *index, &before, after).await?;
                *index += 1;
            }
            Step::SeekAnimations { milliseconds } => {
                let expression = format!(
                    "document.getAnimations().forEach(a=>{{a.pause();a.currentTime={milliseconds}}})"
                );
                self.browser.cdp.evaluate(&expression).await?;
                self.browser
                    .cdp
                    .evaluate("new Promise(r => requestAnimationFrame(r))")
                    .await?;
                self.transition_state = None;
                self.push(scenario, *index).await?;
                self.clean = false;
                *index += 1;
            }
        }
        Ok(())
    }
    async fn reset(&mut self, index: &mut usize) -> anyhow::Result<()> {
        let source_fixture = self.browser.has_state_fixture();
        if self.clean && !source_fixture {
            *index += 1;
            return Ok(());
        }
        if !source_fixture {
            let graph_clean = self
                .transition_state
                .as_ref()
                .map(transition::state_digest)
                .transpose()?
                .as_deref()
                == Some(&self.baseline_graph_digest);
            if graph_clean {
                replay::neutralize(self.browser).await?;
                collector_transition::refresh(self).await?;
                if self.clean {
                    *index += 1;
                    return Ok(());
                }
            }
        } else {
            tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        }
        self.browser.restore_storage_fixture().await?;
        if source_fixture {
            self.browser
                .cdp
                .send(
                    "Page.navigate",
                    serde_json::json!({"url":self.baseline_url}),
                )
                .await?;
            wait_interaction_ready(self.browser).await?;
        } else {
            reload(self.browser).await?;
        }
        collector_transition::wait_reset(self).await?;
        *index += 1;
        Ok(())
    }

    async fn push_responsive(&mut self, scenario: &Scenario, index: usize) -> anyhow::Result<()> {
        let key = (self.viewport.width, self.viewport.height);
        if let Some(cached) = self.responsive_cache.get(&key) {
            self.checkpoints.push(Checkpoint {
                scenario: scenario.id.clone(),
                step: index,
                viewport: self.viewport.clone(),
                domains: cached.domains.clone(),
            });
            return Ok(());
        }
        let captured = checkpoint::capture(
            &mut self.browser.cdp,
            &scenario.id,
            index,
            self.viewport.clone(),
        )
        .await?;
        self.responsive_cache.insert(key, captured.clone());
        self.checkpoints.push(captured);
        Ok(())
    }
    async fn push(&mut self, scenario: &Scenario, index: usize) -> anyhow::Result<()> {
        self.checkpoints.push(
            checkpoint::capture(
                &mut self.browser.cdp,
                &scenario.id,
                index,
                self.viewport.clone(),
            )
            .await?,
        );
        Ok(())
    }
}
