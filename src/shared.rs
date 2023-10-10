use pgx::shmem::{PGXSharedMemory, PgSharedMemoryInitialization};

pub struct GraphSharedMemory {
    graph: MyGraph,
    node_names: HashMap<String, NodeId>,
}

unsafe impl PGXSharedMemory for GraphSharedMemory { }

impl PgSharedMemoryInitialization for GraphSharedMemory {
    fn pg_init(&'static self) {
        // Инициализация при загрузке расширения
    }

    fn shmem_init(&'static self) {
        // Инициализация при инициализации системы Shared Memory PostgreSQL
    }
}

pg_shmem! {
    pub static ref GRAPH_SHARED: PgSharedMem<GraphSharedMemory> = PgSharedMem::new(|| GraphSharedMemory {
        graph: MyGraph::new(),
        node_names: HashMap::new(),
    });
}