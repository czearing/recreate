use crate::{
    browser::Browser,
    checkpoint,
    collector_browser::{advance, reload, resize},
    collector_state::state,
    model::{Checkpoint, Scenario, Step, Viewport},
    replay,
};
use std::collections::BTreeMap;
use tokio::time::{Duration, sleep};

pub(crate) struct Run<'a> {
    browser: &'a mut Browser,
    viewport: Viewport,
    checkpoints: Vec<Checkpoint>,
    responsive_cache: BTreeMap<(u32, u32), Checkpoint>,
    baseline_state: String,
    clean: bool,
}

impl<'a> Run<'a> {
    pub(crate) async fn new(browser: &'a mut Browser) -> anyhow::Result<Self> {
        let baseline_state = state(browser).await?;
        Ok(Self {
            browser,
            viewport: Viewport {
                width: 1280,
                height: 800,
            },
            checkpoints: Vec::new(),
            responsive_cache: BTreeMap::new(),
            baseline_state,
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
                self.push_responsive(scenario, *index).await?;
                *index += 1;
            }
            Step::ResizePath { widths, height } => {
                for width in widths {
                    self.viewport = resize(self.browser, *width, *height).await?;
                    self.push_responsive(scenario, *index).await?;
                    *index += 1;
                }
            }
            Step::AdvanceTime { milliseconds } => {
                advance(self.browser, *milliseconds).await?;
                self.push(scenario, *index).await?;
                self.clean = false;
                *index += 1;
            }
            Step::Activate { anchor } => {
                replay::activate(self.browser, anchor).await?;
                self.push(scenario, *index).await?;
                self.clean = false;
                *index += 1;
            }
            Step::Hover { anchor } => {
                replay::hover(self.browser, anchor).await?;
                self.clean = false;
                *index += 1;
            }
            Step::Key { key } => {
                replay::key(self.browser, key).await?;
                self.push(scenario, *index).await?;
                self.clean = self.checkpoints.last().is_some_and(|checkpoint| {
                    self.responsive_cache
                        .get(&(self.viewport.width, self.viewport.height))
                        .is_some_and(|baseline| equivalent_state(checkpoint, baseline))
                });
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
                self.push(scenario, *index).await?;
                self.clean = false;
                *index += 1;
            }
        }
        Ok(())
    }

    async fn reset(&mut self, index: &mut usize) -> anyhow::Result<()> {
        if self.clean || state(self.browser).await? == self.baseline_state {
            self.clean = true;
            *index += 1;
            return Ok(());
        }

        reload(self.browser).await?;
        let mut current = String::new();
        for _ in 0..80 {
            current = state(self.browser).await?;
            if current == self.baseline_state {
                self.clean = true;
                *index += 1;
                return Ok(());
            }
            sleep(Duration::from_millis(25)).await;
        }
        let baseline: serde_json::Value = serde_json::from_str(&self.baseline_state)?;
        let current: serde_json::Value = serde_json::from_str(&current)?;
        let (path, expected, actual) = crate::compare_difference::between(&baseline, &current);
        anyhow::bail!(
            "reset did not restore browser state at {path}: expected={expected} actual={actual}"
        );
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

fn equivalent_state(current: &Checkpoint, baseline: &Checkpoint) -> bool {
    current.domains["structure"].digest == baseline.domains["structure"].digest
}
