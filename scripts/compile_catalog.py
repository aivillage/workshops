#!/usr/bin/env python3
"""Compile workshop.yaml files into catalog.json single source of truth."""

import argparse
import json
import sys
from pathlib import Path
import yaml


def compile_catalog(repo_root: Path) -> dict:
    workshops = {}
    models = set()
    images = set()
    containers = []

    for item in sorted(repo_root.iterdir()):
        if not item.is_dir() or item.name.startswith((".", "_")) or item.name in ("scripts", "build", "dist", "node_modules", "result"):
            continue

        # Check for container target at root of challenge folder
        if (item / "Dockerfile").is_file():
            containers.append({
                "name": item.name,
                "path": str(item.relative_to(repo_root)),
            })

        # Check for container targets in immediate subdirectories
        for sub in sorted(item.iterdir()):
            if sub.is_dir() and not sub.name.startswith((".", "_")):
                if (sub / "Dockerfile").is_file():
                    containers.append({
                        "name": f"{item.name}-{sub.name}",
                        "path": str(sub.relative_to(repo_root)),
                    })

        workshop_yaml = item / "workshop.yaml"
        if workshop_yaml.is_file():
            with open(workshop_yaml, "r", encoding="utf-8") as f:
                data = yaml.safe_load(f)
            if not isinstance(data, dict) or "name" not in data:
                continue

            name = data["name"]
            workshops[name] = data

            # Collect non-empty model
            model = data.get("model")
            if model:
                models.add(model)

            # Collect user.image
            user_cfg = data.get("user")
            if isinstance(user_cfg, dict) and user_cfg.get("image"):
                images.add(user_cfg["image"])

            # Collect service.image
            service_cfg = data.get("service")
            if isinstance(service_cfg, dict) and service_cfg.get("image"):
                images.add(service_cfg["image"])

    return {
        "workshops": dict(sorted(workshops.items())),
        "workshopInfo": {
            "requiredModels": sorted(list(models)),
            "requiredImages": sorted(list(images)),
        },
        "containers": sorted(containers, key=lambda c: c["name"]),
    }


def main():
    parser = argparse.ArgumentParser(
        description="Compile workshop.yaml files into catalog.json"
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Check if catalog.json is in sync without modifying it",
    )
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parent.parent
    catalog_file = repo_root / "catalog.json"

    catalog = compile_catalog(repo_root)
    new_content = json.dumps(catalog, indent=2) + "\n"

    if args.check:
        if not catalog_file.exists():
            print(
                f"Error: {catalog_file} does not exist. Run scripts/compile_catalog.py to generate it.",
                file=sys.stderr,
            )
            sys.exit(1)
        existing_content = catalog_file.read_text(encoding="utf-8")
        if existing_content != new_content:
            print(
                f"Error: {catalog_file} is out of sync with workshop.yaml files. Run scripts/compile_catalog.py to update it.",
                file=sys.stderr,
            )
            sys.exit(1)
        print("catalog.json is in sync.")
        sys.exit(0)
    else:
        catalog_file.write_text(new_content, encoding="utf-8")
        print(f"Wrote {catalog_file}")


if __name__ == "__main__":
    main()
