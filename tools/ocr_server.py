"""OCR MCP Server - Tesseract wrapper for OpenCode."""

import os
import sys
import base64
import tempfile
from pathlib import Path

from mcp.server import Server
from mcp.server.stdio import stdio_server
from mcp.types import Tool, TextContent

TESSERACT_EXE = r"C:\Program Files\Tesseract-OCR\tesseract.exe"
TESSDATA_DIR = r"D:\tessdata"
DEFAULT_LANG = "chi_sim+eng"

os.environ["TESSDATA_PREFIX"] = TESSDATA_DIR

server = Server("ocr")


@server.list_tools()
async def list_tools():
    return [
        Tool(
            name="ocr",
            description=(
                "Extract text from an image file using Tesseract OCR. "
                "Supports Chinese (chi_sim) and English (eng). "
                "Accepts an image file path and returns the recognized text."
            ),
            inputSchema={
                "type": "object",
                "properties": {
                    "image_path": {
                        "type": "string",
                        "description": "Absolute path to the image file (png, jpg, bmp, tiff, etc.)",
                    },
                    "lang": {
                        "type": "string",
                        "description": f"Tesseract language code (default: {DEFAULT_LANG})",
                        "default": DEFAULT_LANG,
                    },
                },
                "required": ["image_path"],
            },
        )
    ]


@server.call_tool()
async def call_tool(name: str, arguments: dict):
    if name != "ocr":
        raise ValueError(f"Unknown tool: {name}")

    image_path = arguments.get("image_path", "")
    lang = arguments.get("lang", DEFAULT_LANG)

    if not image_path:
        return [TextContent(type="text", text="Error: image_path is required")]

    path = Path(image_path)
    if not path.exists():
        return [TextContent(type="text", text=f"Error: file not found: {image_path}")]

    if path.suffix.lower() not in (".png", ".jpg", ".jpeg", ".bmp", ".tiff", ".tif", ".webp"):
        return [TextContent(type="text", text=f"Error: unsupported image format: {path.suffix}")]

    if not os.path.isfile(TESSERACT_EXE):
        return [TextContent(type="text", text=f"Error: Tesseract not found at {TESSERACT_EXE}")]

    try:
        import pytesseract
        from PIL import Image

        pytesseract.pytesseract.tesseract_cmd = TESSERACT_EXE
        img = Image.open(path)
        text = pytesseract.image_to_string(img, lang=lang)

        if not text or not text.strip():
            return [TextContent(type="text", text="OCR completed but no text was recognized.")]

        return [TextContent(type="text", text=text)]

    except Exception as e:
        return [TextContent(type="text", text=f"OCR error: {e}")]


async def main():
    async with stdio_server() as (read, write):
        await server.run(read, write, server.create_initialization_options())


if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
