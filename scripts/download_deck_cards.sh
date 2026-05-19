#!/bin/bash

# Download only the card data for specific card IDs from the deck

set -e

OUTPUT_DIR="assets/card_data"
mkdir -p "$OUTPUT_DIR"

# Card IDs from the deck
CARD_IDS=(
    66431519 23015896 44455560 90681088 18621798 02526224 14558127 97268402 24508238 33854624
    06637331 65305978 57554544 91703676 84211599 49238328 24224830 10045474 40366667
    94259633 60303245 87871125 02772337 48815792 08264361 29301450 93039339 64182380
)

echo "Downloading ${#CARD_IDS[@]} unique card files..."

for id in "${CARD_IDS[@]}"; do
    url="https://raw.githubusercontent.com/DawnbrandBots/yaml-yugi/master/data/cards/${id}.json"
    output_file="$OUTPUT_DIR/${id}.json"
    
    if [ ! -f "$output_file" ]; then
        curl -s "$url" -o "$output_file" 2>/dev/null || echo "Failed: $id"
    fi
done

echo "Done! Card data saved to $OUTPUT_DIR/"
echo "Downloaded $(ls -1 $OUTPUT_DIR/*.json 2>/dev/null | wc -l) card files"
