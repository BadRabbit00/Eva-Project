use crate::context_engine::ContextEngine;
use crate::scheduler::TaskNode;
use shared_ipc::eva::eva_server::Eva;
use shared_ipc::eva::{
    QueueRequest, QueueResponse, RegistryRequest, RegistryResponse, SubmitRequest, SubmitResponse,
    TaskStatusEvent, TaskStatusRequest,
};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tonic::{Request, Response, Status};

pub struct EvaService {
    pub task_sender: mpsc::Sender<TaskNode>,
    pub context_engine: Arc<RwLock<ContextEngine>>,
    pub worker_manager: Arc<RwLock<crate::worker_manager::WorkerManager>>,
}

#[tonic::async_trait]
impl Eva for EvaService {
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
        tracing::info!("Stream requested for task: {}", req.task_id);

        let (tx, rx) = mpsc::channel(128);
        let wm_clone = self.worker_manager.clone();

        tokio::spawn(async move {
            let mut local_tail = 0;
            loop {
                let wm = wm_clone.read().await;
                if let Some(agent) = wm.agents.get("zero-node") {
                    let rb = agent.shmem_manager.output_ring_buffer();
                    let head = rb.head.load(std::sync::atomic::Ordering::Acquire);

                    if head > local_tail {
                        let mut buf = Vec::new();
                        while local_tail < head {
                            buf.push(
                                rb.buffer
                                    [local_tail % shared_ipc::memory_map::RING_BUFFER_CAPACITY],
                            );
                            local_tail += 1;
                        }

                        let text = String::from_utf8_lossy(&buf).to_string();
                        if tx
                            .send(Ok(TaskStatusEvent {
                                node_id: "zero-node".into(),
                                event_type: "TOKEN".into(),
                                content: text,
                            }))
                            .await
                            .is_err()
                        {
                            break; // Client disconnected
                        }
                    }

                    // Check if done
                    let status = agent.shmem_manager.read_status();
                    if status == Some(shared_ipc::protocol::WorkerStatus::Done)
                        || status == Some(shared_ipc::protocol::WorkerStatus::Error)
                    {
                        if local_tail == rb.head.load(std::sync::atomic::Ordering::Acquire) {
                            break;
                        }
                    }
                } else {
                    break;
                }
                drop(wm);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        });

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

#[cfg(test)]
mod tests {
    use super::*;
    use shared_ipc::eva::SubmitRequest;
    use tonic::Request;

    #[tokio::test]
    async fn test_submit_task_grpc() {
        let (tx, mut rx) = mpsc::channel(10);
        let ctx = Arc::new(RwLock::new(ContextEngine::new(std::path::PathBuf::from(
            "/tmp",
        ))));

        let service = EvaService {
            task_sender: tx,
            context_engine: ctx,
            worker_manager: Arc::new(RwLock::new(crate::worker_manager::WorkerManager::new(
                1024,
                "/tmp".into(),
            ))),
        };

        let req = Request::new(SubmitRequest {
            prompt: "Analyze the logs".into(),
            template_id: "sys_debugger".into(),
            priority: 9,
        });

        let response = service.submit_task(req).await.unwrap().into_inner();
        assert_eq!(response.status, "QUEUED");

        // Ensure the task was pushed to the channel
        let task = rx.recv().await.expect("Task should be in channel");
        assert_eq!(task.priority, 9);
        assert_eq!(task.id, response.task_id);
    }
}
