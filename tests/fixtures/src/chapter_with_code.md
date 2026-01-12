# Code Examples

The API provides several endpoints. Here's an example:

```rust
// This API call should not be linked
let response = client.get("/api/users").send()?;
let data: JSON = response.json()?;
```

Inline code like `REST` and `API` should not be linked.

## JSON Parsing

Parse JSON data using the built-in parser:

```python
import json

# JSON is used here but should not be linked
data = json.loads('{"name": "test"}')
```

The CLI tool is available for testing. See [API Documentation](https://example.com) for more details.
