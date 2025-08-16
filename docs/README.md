# Readur Documentation

This directory contains the source files for the Readur documentation site, built with MkDocs and Material for MkDocs.

## Local Development

### Prerequisites

- Python 3.8+
- pip

### Setup

1. Install dependencies:
```bash
pip install -r ../requirements.txt
```

2. Start the development server:
```bash
mkdocs serve
```

The documentation will be available at `http://localhost:8000`.

### Building

To build the static site:
```bash
mkdocs build
```

The built site will be in the `site/` directory.

## Deployment

The documentation is automatically deployed to [readur.app](https://readur.app) via GitHub Actions when changes are pushed to the main branch.

### Manual Deployment

If you need to deploy manually:

1. Build the site:
```bash
mkdocs build
```

2. Deploy to Cloudflare Pages:
```bash
wrangler pages deploy site --project-name=readur-docs
```

## Structure

- `docs/`  
  Documentation source files (Markdown)
  
- `mkdocs.yml`  
  MkDocs configuration
  
- `requirements.txt`  
  Python dependencies
  
- `overrides/`  
  Theme customizations
  
- `stylesheets/`  
  Custom CSS
  
- `javascripts/`  
  Custom JavaScript

## Writing Documentation

### Adding New Pages

1. Create a new `.md` file in the appropriate directory
2. Add the page to the navigation in `mkdocs.yml`
3. Use Material for MkDocs features for rich content

### Markdown Extensions

We use several markdown extensions for enhanced functionality:

- **Admonitions**  
  For notes, warnings, tips
  
- **Code blocks**  
  With syntax highlighting
  
- **Tabs**  
  For grouped content
  
- **Tables**  
  For structured data
  
- **Emoji**  
  For visual elements

Example:
```markdown
!!! note "Important"
    This is an important note.

=== "Tab 1"
    Content for tab 1

=== "Tab 2"
    Content for tab 2
```

## Contributing

Please follow these guidelines when contributing to the documentation:

1. Use clear, concise language
2. Include code examples where appropriate
3. Test all links and code samples
4. Run `mkdocs build --strict` before submitting
5. Update the navigation in `mkdocs.yml` for new pages

## Resources

- [MkDocs Documentation](https://www.mkdocs.org/)
- [Material for MkDocs](https://squidfunk.github.io/mkdocs-material/)
- [Markdown Guide](https://www.markdownguide.org/)