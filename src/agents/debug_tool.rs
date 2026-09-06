//! Debug wrapper for Rig tools to provide real-time observability

use anyhow::Result;
use rig::tool::portable::PortableTool;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::marker::PhantomData;

use crate::agents::debug;

/// Wrapper that adds debug logging to any Rig tool
pub struct DebugTool<T>
where
    T: PortableTool,
{
    inner: T,
    _phantom: PhantomData<T>,
}

impl<T> DebugTool<T>
where
    T: PortableTool,
{
    pub fn new(tool: T) -> Self {
        Self {
            inner: tool,
            _phantom: PhantomData,
        }
    }
}

impl<T> Clone for DebugTool<T>
where
    T: PortableTool + Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<T> Debug for DebugTool<T>
where
    T: PortableTool + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebugTool")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<T> Serialize for DebugTool<T>
where
    T: PortableTool + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for DebugTool<T>
where
    T: PortableTool + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner = T::deserialize(deserializer)?;
        Ok(Self {
            inner,
            _phantom: PhantomData,
        })
    }
}

impl<T> PortableTool for DebugTool<T>
where
    T: PortableTool + Send + Sync,
    T::Args: Debug + Send + Sync,
    T::Output: Debug + Send + Sync,
    T::Error: Send + Sync,
{
    const NAME: &'static str = T::NAME;
    type Error = T::Error;
    type Args = T::Args;
    type Output = T::Output;

    fn description(&self) -> String {
        self.inner.description()
    }

    fn parameters(&self) -> serde_json::Value {
        self.inner.parameters()
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Debug output before tool execution
        let args_str = format!("{:?}", args);
        debug::debug_tool_call(Self::NAME, &args_str);

        // Start timer
        let timer = debug::DebugTimer::start(&format!("Tool: {}", Self::NAME));

        // Call the actual tool
        let result = self.inner.call(args).await;

        // Finish timer
        timer.finish();

        // Debug output after tool execution
        match &result {
            Ok(output) => {
                let output_str = format!("{:?}", output);
                debug::debug_tool_response(
                    Self::NAME,
                    &output_str,
                    std::time::Duration::from_secs(0), // Timer already printed
                );
            }
            Err(e) => {
                debug::debug_error(&format!("Tool {} failed: {:?}", Self::NAME, e));
            }
        }

        result
    }
}
