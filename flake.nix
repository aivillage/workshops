{
  description = "A flake to build and push Docker containers to GHCR";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      # Define supported systems for cross-platform compatibility
      supportedSystems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      devShells = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};

          # --- Script Generation Logic ---
          mkContainerScripts = containers:
            let
              scriptPreamble = ''
                #!/usr/bin/env bash
                set -euo pipefail # Exit on error, unset variables, and pipe failures
                
                # Check for project root
                if [[ -z "''${PROJECT_ROOT:-}" ]]; then
                  echo "Error: PROJECT_ROOT is not set. Are you in the dev shell?"
                  exit 1
                fi
              '';
              
              mkOneScript = { name, path, type ? "docker" }:
                let
                  scriptName = "upload-${name}";
                  
                  dockerBody = ''
                    echo "--- Processing image: ${name} ---"
                    LOCAL_TAG="${name}:latest"
                    REMOTE_TAG="ghcr.io/nbhdai/${name}:latest"
                    CONTEXT_PATH="$PROJECT_ROOT/${path}"

                    echo "Building $LOCAL_TAG from $CONTEXT_PATH..."
                    docker build -t "$LOCAL_TAG" "$CONTEXT_PATH"
                    
                    echo "Tagging $LOCAL_TAG as $REMOTE_TAG..."
                    docker tag "$LOCAL_TAG" "$REMOTE_TAG"
                    
                    echo "Pushing $REMOTE_TAG..."
                    docker push "$REMOTE_TAG"
                    echo "Successfully pushed $REMOTE_TAG"
                    echo "-----------------------------------"
                  '';

                  nixBody = ''
                    echo "--- Processing image: ${name} ---"
                    LOCAL_TAG="${name}:latest"
                    REMOTE_TAG="ghcr.io/nbhdai/${name}:latest"
                    RESULT_LINK="result-${name}"

                    echo "Building Nix attribute: ${path}..."
                    nix build "$PROJECT_ROOT#${path}" --out-link "$RESULT_LINK"
                    
                    echo "Loading $LOCAL_TAG into Docker..."
                    docker load < "$RESULT_LINK"
                    
                    echo "Tagging $LOCAL_TAG as $REMOTE_TAG..."
                    docker tag "$LOCAL_TAG" "$REMOTE_TAG"
                    
                    echo "Pushing $REMOTE_TAG..."
                    docker push "$REMOTE_TAG"
                    
                    rm "$RESULT_LINK"
                    echo "Successfully pushed $REMOTE_TAG"
                    echo "-----------------------------------"
                  '';
                  
                  scriptBody = if type == "docker" then dockerBody else nixBody;
                in
                  pkgs.writeShellScriptBin scriptName (scriptPreamble + scriptBody);

              mkAllScript = containers:
                let
                  calls = map (c: ''
                    echo "Triggering upload-${c.name}..."
                    upload-${c.name}
                  '') containers;
                in
                  pkgs.writeShellScriptBin "upload-all-images" ''
                    ${scriptPreamble}
                    echo "=== 🚀 Starting upload for all images... ==="
                    echo ""
                    ${builtins.concatStringsSep "\n" calls}
                    echo ""
                    echo "=== ✅ All images pushed successfully! ==="
                  '';

            in
              (map mkOneScript containers) ++ [ (mkAllScript containers) ];

          # --- Container Definitions ---
          myContainers = [
            { name = "yolo-l2-notebook";     path = "yolo-l2/notebook"; }
            { name = "yolo-l2-verification"; path = "yolo-l2/verification"; }
            { name = "email-indirect-service"; path = "email-indirect/service"; }
            { name = "email-indirect-user";    path = "email-indirect/user"; }
          ];

          # Generate the packages wrapping your bash scripts
          myContainerScripts = mkContainerScripts myContainers;

        in
        {
          default = pkgs.mkShell {
            # Inject Docker and your generated scripts into the shell environment
            buildInputs = with pkgs; [
              docker
            ] ++ myContainerScripts;

            # ShellHook runs automatically when entering `nix develop`
            shellHook = ''
              export PROJECT_ROOT=$(pwd)
              
              echo "📦 Container Build Shell Initialized"
              echo "------------------------------------"
              echo "Available commands:"
              echo "  upload-all-images"
              ${builtins.concatStringsSep "\n" (map (c: "echo \"  upload-${c.name}\"") myContainers)}
              echo ""
              echo "⚠️  Ensure you are logged into GHCR before pushing:"
              echo "  echo \$CR_PAT | docker login ghcr.io -u YOUR_GITHUB_USERNAME --password-stdin"
            '';
          };
        });
    };
}