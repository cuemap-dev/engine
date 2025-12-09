# CueMap Rust Engine

**High-performance temporal-associative memory store** that mimics the brain's recall mechanism.

## Overview

CueMap implements a memory system inspired by how the human brain recalls information:
- **Temporal**: Recent memories are more accessible
- **Associative**: Multiple cues trigger stronger recall
- **Reinforcement**: Frequently accessed memories stay "front of mind"

Built with Rust for maximum performance and reliability.

## Quick Start

### Build & Run

```bash
# Development
cargo run

# Production (optimized)
cargo build --release
./target/release/cuemap-rust --port 8080
```

### Docker

```bash
docker build -f Dockerfile.production -t cuemap/engine .
docker run -p 8080:8080 cuemap/engine
```

### CLI Options

```bash
./target/release/cuemap-rust --help

Options:
  -p, --port <PORT>                    Server port [default: 8080]
  -d, --data-dir <DATA_DIR>            Data directory [default: ./data]
  -s, --snapshot-interval <SECONDS>    Snapshot interval [default: 60]
  -m, --multi-tenant                   Enable multi-tenancy
```

## Performance

### Benchmark Results

Tested on realistic workloads with Zipfian distribution (80% of operations hit 20% of cues):

#### Write Performance

| Dataset | Avg Latency | P99 Latency | Throughput |
|---------|-------------|-------------|------------|
| 100K    | 0.19ms      | 0.30ms      | 3,067 ops/s |
| 1M      | 0.20ms      | 0.33ms      | 2,926 ops/s |

#### Read Performance

| Dataset | Avg Latency | P50 Latency | P99 Latency | Throughput |
|---------|-------------|-------------|-------------|------------|
| 100K    | 0.23ms      | 0.22ms      | 0.35ms      | 2,782 ops/s |
| 1M      | 0.23ms      | 0.22ms      | 0.37ms      | 2,763 ops/s |
| 10M     | 1.40ms      | 1.20ms      | 3.90ms      | 700 ops/s |

**Key Metrics**:
- ✅ **Sub-millisecond P99 latency** at 1M scale
- ✅ **Sub-5ms P99 latency** at 10M scale (production-tested)
- ✅ **Consistent performance** across dataset sizes
- ✅ **2,900+ ops/sec** sustained throughput (1M)
- ✅ **700+ queries/sec** at 10M scale

**Memory Efficiency**:
- ✅ **~500 bytes per memory** (content + cues + indexes)
- ✅ **5 GB RAM for 10M memories** (production-tested)
- ✅ **Linear scaling** with dataset size

### Correctness Tests

Validated on 120+ test scenarios:
- ✅ **Recency**: 30/30 (100%) - Recent memories prioritized
- ✅ **Intersection**: 30/30 (100%) - Multi-cue matching works
- ✅ **Reinforcement**: 20/20 (100%) - Move-to-front operation
- ✅ **Multi-Cue**: 20/20 (100%) - Complex queries
- ✅ **Noise Filtering**: 20/20 (100%) - Irrelevant memories filtered

### Concurrent Performance

Stress tested with 400+ parallel operations:
- ✅ **100% success rate** under concurrent load
- ✅ **100% recall accuracy** with parallel reads/writes
- ✅ **Lock-free operations** with DashMap

## Architecture

### Core Components

- **Axum**: Minimal overhead async web framework
- **DashMap**: Lock-free concurrent hash map (32 shards)
- **IndexSet**: O(1) move-to-front operations
- **Bincode**: Fast binary serialization for persistence

### Optimizations

- **Zero-copy**: Efficient memory management with Arc
- **Pre-allocated collections**: Capacity hints eliminate reallocation
- **Unstable sorting**: 2-3x faster than stable sort
- **Iterative deepening**: Early termination on hot paths

## API

### Add Memory

```bash
curl -X POST http://localhost:8080/memories \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Server password is abc123",
    "cues": ["server", "password", "credentials"]
  }'
```

### Recall Memories

```bash
curl -X POST http://localhost:8080/recall \
  -H "Content-Type: application/json" \
  -d '{
    "cues": ["server", "password"],
    "limit": 10,
    "auto_reinforce": false
  }'
```

### Reinforce Memory

```bash
curl -X PATCH http://localhost:8080/memories/{id}/reinforce \
  -H "Content-Type: application/json" \
  -d '{
    "cues": ["important", "urgent"]
  }'
```

### Get Memory

```bash
curl http://localhost:8080/memories/{id}
```

### Get Stats

```bash
curl http://localhost:8080/stats
```

## Production Features

### Persistence

- **Bincode snapshots**: 10x faster than JSON
- **Background saves**: Every 60s (configurable)
- **Atomic writes**: Temp file + rename pattern
- **Graceful shutdown**: SIGINT/SIGTERM handlers

### Authentication

```bash
export CUEMAP_API_KEY="your-secret-key"
./target/release/cuemap-rust
```

### Multi-Tenancy

```bash
./target/release/cuemap-rust --multi-tenant

# Use project-specific endpoints
curl -X POST http://localhost:8080/v1/my-project/memories ...
```

## Monitoring

### Health Check

```bash
curl http://localhost:8080/
```

### Statistics

```bash
curl http://localhost:8080/stats | jq .
```

Returns:
```json
{
  "total_memories": 1000000,
  "total_cues": 1418,
  "cues": ["user", "system", "data", ...]
}
```

## License

AGPLv3 - See LICENSE for details

For commercial licensing (closed-source SaaS), contact: kaandemirel@yahoo.com
