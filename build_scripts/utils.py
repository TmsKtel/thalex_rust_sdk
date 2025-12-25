"""
Utils for building the Thalex Rust SDK from the OpenAPI spec.
"""


def collect_all_tags_from_spec(spec):
    tags = set()
    for _, path_spec in spec["paths"].items():
        path_tags = path_spec.get("get", {}).get("tags", [])
        print("   Found tags:", path_tags)
        for tag in path_tags:
            tags.add(tag)
    return tags