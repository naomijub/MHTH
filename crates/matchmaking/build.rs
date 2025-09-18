    fn main() -> Result<(), Box<dyn std::error::Error>> {
        tonic_prost_build::configure()
            .build_client(true)
            .build_server(true)
            .compile_protos(&["protos/matchmaking.proto"], &["protos"])?;
        Ok(())
    }