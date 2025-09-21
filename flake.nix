{
  description = "A Rust MCP SSE Server development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, fenix, ... }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      rustToolchain = fenix.packages.${system}.fromToolchainFile {
        file = ./rust-toolchain.toml;
        sha256 = "sha256-+9FmLhAOezBZCOziO0Qct1NOrfpjNsXxc/8I0c7BdKE=";
      };
    in
    {
      devShells.${system}.default = pkgs.mkShell
        {
          buildInputs = [
            rustToolchain
            pkgs.rust-analyzer
            pkgs.cargo-expand
            pkgs.cargo-watch
            pkgs.cargo-edit
            pkgs.openssl
            pkgs.pkg-config

          ];

          shellHook = ''
            echo "MCP SSE Server development environment ready!"
            echo "Rust version: $(rustc --version)"
            echo ""
            echo "Quick start:"
            echo "  cargo run                 # Start the MCP SSE server"
            echo "  cargo watch -x run        # Auto-restart on changes"
            echo ""
            echo "Server will be available at: http://127.0.0.1:8000"
            echo "SSE endpoint: /sse"
            echo "Message endpoint: /message"
            echo ""
            echo "Test with MCP Inspector: https://github.com/modelcontextprotocol/inspector"
          '';
        };
    };
}
