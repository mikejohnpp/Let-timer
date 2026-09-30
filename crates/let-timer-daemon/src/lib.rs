use let_timer_core::{
    NewTask,
    Response::{self},
    SortOrder,
    TaskPayload::{self},
    TaskRepository, TaskStatus,
};

pub fn create_commands(repo: &TaskRepository, task: &NewTask) -> Response {
    let result = repo.create(task);
    match result {
        Ok(task) => Response::Ok(TaskPayload::Some(task)),
        Err(error) => Response::Error {
            message: error.to_string(),
        },
    }
}

pub fn get_list_task(
    repo: &TaskRepository,
    sort_by_status: Option<TaskStatus>,
    sort_by_priority: Option<SortOrder>,
) -> Response {
    if let Some(task_status) = sort_by_status {
        let result = repo.list_by_status(task_status);
        match result {
            Ok(list_task) => return Response::OkList(list_task),
            Err(error) => {
                return Response::Error {
                    message: error.to_string(),
                };
            }
        }
    }

    if let Some(sort) = sort_by_priority {
        let result = repo.list_sorted_by_priority(sort);
        match result {
            Ok(list_task) => return Response::OkList(list_task),
            Err(error) => {
                return Response::Error {
                    message: error.to_string(),
                };
            }
        };
    }

    let result = repo.list_all();
    match result {
        Ok(list_task) => Response::OkList(list_task),
        Err(error) => Response::Error {
            message: error.to_string(),
        },
    }
}

pub fn delete_task(repo: &TaskRepository, id: i64) -> Response {
    let result = repo.delete(id);
    match result {
        Ok(_) => Response::OkEmpty,
        Err(error) => Response::Error {
            message: error.to_string(),
        },
    }
}

pub fn find_by_name(repo: &TaskRepository, query: &str) -> Response {
    let result = repo.find_by_name(query);
    match result {
        Ok(list_task) => Response::OkList(list_task),
        Err(error) => Response::Error {
            message: error.to_string(),
        },
    }
}

pub fn current_task(repo: &TaskRepository) -> Response {
    let result = repo.get_current();
    match result {
        Ok(has_task) => match has_task {
            Some(task) => Response::Ok(TaskPayload::Some(task)),
            None => Response::OkEmpty,
        },
        Err(error) => Response::Error {
            message: error.to_string(),
        },
    }
}

pub fn start_task(repo: &TaskRepository, id: Option<i64>) -> Response {
    if let Some(id) = id {
        let result = repo.start_task(id);
        match result {
            Ok(task) => Response::Ok(TaskPayload::Some(task)),
            Err(error) => Response::Error {
                message: error.to_string(),
            },
        }
    } else {
        Response::OkEmpty
    }
}
