# High Frequency Trading Engine

A high-performance trading engine built in Rust, designed for low-latency market making and algorithmic trading strategies.

## Features

- Multi-venue support with unified interface
- Real-time order book management
- Configurable trading strategies
- Smart order routing
- Prometheus metrics integration
- Docker deployment support
- Real-time monitoring with Grafana

## Prerequisites

- Rust 1.75 or higher
- Docker and Docker Compose
- 16GB RAM minimum recommended
- Linux environment recommended for optimal performance

## Quick Start

1. Clone the repository:
```bash
git clone https://github.com/yourusername/hft_engine
cd hft_engine
```

2. Build the project:
```bash
cargo build --release
```

3. Start the monitoring stack:
```bash
docker-compose up -d
```

4. Run the engine:
```bash
./target/release/hft_engine
```

## Project Structure

```
src/
├── book/           # Order book management
├── command/        # Control interface
├── execution/      # Order execution logic
├── gateways/       # Market data and order handling
├── metrics/        # Prometheus metrics
├── services/       # System coordination
├── strategy/       # Trading strategies
├── types.rs        # Core data structures
└── venues/         # Venue integration
```

## Configuration

Configuration is loaded from `config/` directory. Example configuration:

```yaml
venues:
  - name: venue1
    type: binance
    api_key: ${BINANCE_API_KEY}
    api_secret: ${BINANCE_API_SECRET}
    symbols:
      - BTC-USDT
      - ETH-USDT

strategy:
  type: market_making
  parameters:
    spread: 0.002
    max_position: 1.0
    min_profit: 0.0001
```

## Monitoring

- Grafana dashboard: http://localhost:3000
- Prometheus metrics: http://localhost:9090
- Application metrics endpoint: http://localhost:8080/metrics

## Development

### Running Tests
```bash
cargo test
```

### Benchmarking
```bash
cargo bench
```

### Adding a New Venue

1. Implement the `VenueAdapter` trait
2. Add venue-specific error handling
3. Implement market data handling
4. Implement order management

Example:
```rust
#[async_trait]
impl VenueAdapter for NewVenue {
    async fn subscribe_quotes(&self, symbols: Vec<String>) -> Result<(), Error> {
        // Implementation
    }

    async fn submit_order(&self, order: Order) -> Result<String, Error> {
        // Implementation
    }
}
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Commit your changes
4. Push to the branch
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built with Rust 🦀
- Uses Tokio for async runtime
- Prometheus for metrics
- Grafana for visualization