use crate::context_engine::ContextEngine;
use crate::scheduler::TaskNode;
use shared_ipc::eva::hypervisor_server::Hypervisor;
use shared_ipc::eva::{
    QueueRequest, QueueResponse, RegistryRequest, RegistryResponse, SubmitRequest, SubmitResponse,
    TaskStatusEvent, TaskStatusRequest,
};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tonic::{Request, Response, Status};

pub struct HypervisorService {
    pub task_sender: mpsc::Sender<TaskNode>,
    pub context_engine: Arc<RwLock<ContextEngine>>,
}

#[tonic::async_trait]
impl Hypervisor for HypervisorService {
    async fn submit_task(
        &self,
        request: Request<SubmitRequest>,
    ) -> Result<Response<SubmitResponse>, Status> {
        let req = request.into_inner();
        let job_id = uuid::Uuid::new_v4().to_string();

        tracing::info!("Received task submission: {:?}", req);

        let task = TaskNode {
            id: job_id.clone(),
            node_def: crate::router::PipelineNode {
                id: "api_inference".into(),
                node_type: crate::router::NodeType::Inference,
                model: None,
                thinking_mode: false,
                prompt_template: Some(req.prompt),
                depends_on: vec![],
                next: vec![],
            },
            priority: req.priority,
            estimated_time_ms: 1000,
        };

        let _ = self.task_sender.send(task).await;

        Ok(Response::new(SubmitResponse {
            task_id: job_id,
            status: "QUEUED".into(),
        }))
    }

    type StreamTaskStatusStream =
        tokio_stream::wrappers::ReceiverStream<Result<TaskStatusEvent, Status>>;

    async fn stream_task_status(
        &self,
        request: Request<TaskStatusRequest>,
    ) -> Result<Response<Self::StreamTaskStatusStream>, Status> {
        let req = request.into_inner();
        tracing::info!("Stream requested for: {}", req.task_id);

        let (tx, rx) = mpsc::channel(4);
        let _ = tx
            .send(Ok(TaskStatusEvent {
                node_id: "system".into(),
                event_type: "SYS_STATUS".into(),
                content: "Stream connected".into(),
            }))
            .await;

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }

    async fn get_queue(
        &self,
        _request: Request<QueueRequest>,
    ) -> Result<Response<QueueResponse>, Status> {
        Ok(Response::new(QueueResponse {
            json_dump: "{}".into(),
        }))
    }

    async fn get_registries(
        &self,
        _request: Request<RegistryRequest>,
    ) -> Result<Response<RegistryResponse>, Status> {
        Ok(Response::new(RegistryResponse {
            models_yaml: "".into(),
            tools_yaml: "".into(),
        }))
    }
}
