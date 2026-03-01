use agent_client_protocol as acp;
use agent_client_protocol::{Agent, Client};
use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::task::LocalSet;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

#[derive(Default)]
struct FakeAgent {
    client: Mutex<Option<Arc<acp::AgentSideConnection>>>,
    next_session: Mutex<u64>,
    sessions: Mutex<HashMap<acp::SessionId, PathBuf>>,
}

impl FakeAgent {
    fn set_client(&self, conn: Arc<acp::AgentSideConnection>) {
        *self.client.lock().expect("client lock") = Some(conn);
    }

    fn get_client(&self) -> Result<Arc<acp::AgentSideConnection>> {
        self.client
            .lock()
            .expect("client lock")
            .clone()
            .ok_or_else(|| anyhow!("client connection not initialized"))
    }
}

#[async_trait::async_trait(?Send)]
impl Agent for FakeAgent {
    async fn initialize(
        &self,
        args: acp::InitializeRequest,
    ) -> acp::Result<acp::InitializeResponse> {
        Ok(
            acp::InitializeResponse::new(args.protocol_version).agent_info(
                acp::Implementation::new("llman-fake-acp-agent", env!("CARGO_PKG_VERSION"))
                    .title("llman fake ACP agent"),
            ),
        )
    }

    async fn authenticate(
        &self,
        _args: acp::AuthenticateRequest,
    ) -> acp::Result<acp::AuthenticateResponse> {
        Ok(acp::AuthenticateResponse::new())
    }

    async fn new_session(
        &self,
        args: acp::NewSessionRequest,
    ) -> acp::Result<acp::NewSessionResponse> {
        let mut next = self.next_session.lock().expect("next_session lock");
        *next += 1;
        let session_id = acp::SessionId::new(format!("fake-session-{}", *next));
        self.sessions
            .lock()
            .expect("sessions lock")
            .insert(session_id.clone(), args.cwd);
        Ok(acp::NewSessionResponse::new(session_id))
    }

    async fn prompt(&self, args: acp::PromptRequest) -> acp::Result<acp::PromptResponse> {
        let session_id = args.session_id;
        let cwd = self
            .sessions
            .lock()
            .expect("sessions lock")
            .get(&session_id)
            .cloned()
            .ok_or_else(acp::Error::invalid_params)?;

        let client = self
            .get_client()
            .map_err(|e| acp::Error::internal_error().data(e.to_string()))?;

        // Intentionally emit an env var to stderr to test client-side redaction.
        if let Ok(value) = std::env::var("ANTHROPIC_AUTH_TOKEN") {
            eprintln!("ANTHROPIC_AUTH_TOKEN={value}");
        }

        // Normal tool calls: write a file in the workspace and run a simple terminal command.
        let out_path = cwd.join("fake-agent-output.txt");
        let _ = client
            .write_text_file(acp::WriteTextFileRequest::new(
                session_id.clone(),
                out_path,
                "hello from llman-fake-acp-agent\n",
            ))
            .await;

        let _ = client
            .create_terminal(
                acp::CreateTerminalRequest::new(session_id.clone(), "git")
                    .args(vec!["--version".to_string()])
                    .cwd(Some(cwd.clone())),
            )
            .await;

        // Sandbox probe: attempt to read outside workspace.
        let _ = client
            .read_text_file(acp::ReadTextFileRequest::new(
                session_id.clone(),
                PathBuf::from("/etc/passwd"),
            ))
            .await;

        Ok(acp::PromptResponse::new(acp::StopReason::EndTurn))
    }

    async fn cancel(&self, _args: acp::CancelNotification) -> acp::Result<()> {
        Ok(())
    }
}

fn main() -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("create tokio runtime")?;

    let local = LocalSet::new();
    runtime.block_on(local.run_until(async move {
        let agent = Arc::new(FakeAgent::default());

        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (conn, io_task) = acp::AgentSideConnection::new(
            agent.clone(),
            stdout.compat_write(),
            stdin.compat(),
            |fut| {
                tokio::task::spawn_local(fut);
            },
        );

        agent.set_client(Arc::new(conn));

        io_task.await.map_err(|e| anyhow!("io task failed: {e}"))?;
        Ok(())
    }))
}
