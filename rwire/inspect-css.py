#!/usr/bin/env python3
"""
Inspect CSS injected via WebSocket STYLE_INJECT opcode.

This script connects to a rwire server, receives the initial WebSocket message,
and extracts/displays the CSS that's injected via the STYLE_INJECT opcode.
"""

import asyncio
import websockets
import sys

# Opcode constants
STYLE_INJECT = 0x80


def parse_varint(data, offset):
    """Parse a variable-length integer from the data."""
    b = data[offset]
    if b < 0x80:
        return b, 1
    if b < 0xC0:
        return 0x80 + ((b & 0x3F) << 8) + data[offset + 1], 2
    return 0x4080 + ((b & 0x3F) << 16) + (data[offset + 1] << 8) + data[offset + 2], 3


def extract_css_from_message(data):
    """Extract CSS from a WebSocket message containing STYLE_INJECT opcode."""
    i = 0
    css_blocks = []

    while i < len(data):
        opcode = data[i]
        i += 1

        if opcode == STYLE_INJECT:
            # STYLE_INJECT format: [0x80, length_hi, length_lo, css_bytes...]
            length = (data[i] << 8) | data[i + 1]
            i += 2

            css = data[i:i + length].decode('utf-8')
            css_blocks.append(css)
            i += length
        elif opcode == 0xFF:  # BATCH_END
            break
        else:
            # Skip other opcodes - just trying to find STYLE_INJECT
            # This is a simplified parser, real parsing would need full opcode handling
            try:
                # Try to skip ahead safely
                if opcode == 0xF0:  # SYMBOLS
                    count, vlen = parse_varint(data, i)
                    i += vlen
                    for _ in range(count):
                        sym_len = data[i]
                        i += 1 + sym_len
                elif opcode in [0x01, 0x02, 0x03, 0x05, 0x10, 0x11, 0x13, 0x14, 0x15]:
                    # Opcodes with varint args
                    _, vlen = parse_varint(data, i)
                    i += vlen
                else:
                    # Skip unknown opcodes - advance carefully
                    i += 1
            except (IndexError, Exception):
                break

    return '\n'.join(css_blocks)


async def inspect_css(url='ws://127.0.0.1:9000'):
    """Connect to rwire WebSocket and extract CSS."""
    try:
        async with websockets.connect(url) as websocket:
            print(f"Connected to {url}")

            # Receive the first message (should contain STYLE_INJECT)
            message = await websocket.recv()
            print(f"Received message: {len(message)} bytes")

            if isinstance(message, bytes):
                # Extract CSS from the message
                css = extract_css_from_message(message)

                if css:
                    print(f"\n{'='*60}")
                    print("EXTRACTED CSS:")
                    print(f"{'='*60}\n")
                    print(css)
                    print(f"\n{'='*60}")
                    print(f"CSS size: {len(css)} bytes")

                    # Check for specific variables
                    print(f"\n{'='*60}")
                    print("VARIABLE CHECKS:")
                    print(f"{'='*60}\n")

                    vars_to_check = [
                        '--rw-neutral-1',
                        '--rw-blue-9',
                        '--rw-bg-app',
                        '--rw-text-default',
                        '--rw-space-4',
                        '--rw-radius-md',
                        '--rw-leading-normal',
                        '--rw-white',
                    ]

                    for var in vars_to_check:
                        if f'{var}:' in css:
                            # Extract the value
                            start = css.find(f'{var}:')
                            end = css.find(';', start)
                            if end != -1:
                                value = css[start:end+1]
                                print(f"✓ {value}")
                        else:
                            print(f"✗ {var}: NOT FOUND")
                else:
                    print("No CSS found in message")
            else:
                print(f"Unexpected message type: {type(message)}")

    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    url = sys.argv[1] if len(sys.argv) > 1 else 'ws://127.0.0.1:9000'
    asyncio.run(inspect_css(url))
