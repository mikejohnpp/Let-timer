#!/usr/bin/env bash
set -euo pipefail

# Demo seed script for Let-timer web app
API_URL="${API_URL:-http://127.0.0.1:8765}"
TOKEN="${TOKEN:-}"

headers=(-H "Content-Type: application/json" -H "X-Let-Timer-Protocol: 1")
if [[ -n "$TOKEN" ]]; then
  headers+=(-H "Authorization: Bearer $TOKEN")
fi

echo "Seeding demo data into $API_URL..."

# 1. Create Media items
echo "Creating media items..."
curl -s -X POST "$API_URL/api/v1/medias" "${headers[@]}" -d '{
  "name": "Misty Morning Forest",
  "url": "https://images.unsplash.com/photo-1448375240586-882707db888b?auto=format&fit=crop&w=1200&q=80",
  "media_type": "image",
  "description": "Deep pine forest shrouded in morning mist"
}' > /dev/null || true

curl -s -X POST "$API_URL/api/v1/medias" "${headers[@]}" -d '{
  "name": "Gentle Rain",
  "url": "https://cdn.freesound.org/previews/531/531947_11861866-lq.mp3",
  "media_type": "music",
  "description": "Continuous soothing rain sound"
}' > /dev/null || true

curl -s -X POST "$API_URL/api/v1/medias" "${headers[@]}" -d '{
  "name": "The Road Not Taken",
  "url": "urn:poem:frost",
  "media_type": "poem",
  "description": "Poem by Robert Frost (1916)",
  "content": "Two roads diverged in a yellow wood,\nAnd sorry I could not travel both\nAnd be one traveler, long I stood\nAnd looked down one as far as I could\nTo where it bent in the undergrowth;\n\nThen took the other, as just as fair,\nAnd having perhaps the better claim,\nBecause it was grassy and wanted wear;\nThough as for that the passing there\nHad worn them really about the same,\n\nI shall be telling this with a sigh\nSomewhere ages and ages hence:\nTwo roads diverged in a wood, and I—\nI took the one less traveled by,\nAnd that has made all the difference."
}' > /dev/null || true

# 2. Create Media List
echo "Creating media list..."
ml_res=$(curl -s -X POST "$API_URL/api/v1/media-lists" "${headers[@]}" -d '{
  "name": "Deep Study Sanctuary",
  "description": "Ambient rain and timeless literature for high-focus sessions"
}' || echo "{}")

ml_id=$(echo "$ml_res" | grep -o '"id":[0-9]*' | head -n1 | cut -d: -f2 || echo "1")

# Attach media items to list
for mid in 1 2 3; do
  curl -s -X POST "$API_URL/api/v1/media-lists/$ml_id/medias" "${headers[@]}" -d "{\"media_id\": $mid}" > /dev/null || true
done

# 3. Create Tasks
echo "Creating tasks..."
today=$(date +%Y-%m-%d)

# Task 1: Focus task
t1_res=$(curl -s -X POST "$API_URL/api/v1/tasks" "${headers[@]}" -d "{
  \"name\": \"Read Chapter 4: Distributed Consensus\",
  \"estimated_mins\": 25,
  \"scheduled_on\": \"$today\",
  \"priority\": 2,
  \"media_list_id\": $ml_id
}" || echo "{}")
t1_id=$(echo "$t1_res" | grep -o '"id":[0-9]*' | head -n1 | cut -d: -f2 || echo "1")

# Task 2: Immediate task
curl -s -X POST "$API_URL/api/v1/tasks" "${headers[@]}" -d "{
  \"name\": \"Review pull request #42\",
  \"estimated_mins\": 15,
  \"scheduled_on\": \"$today\",
  \"priority\": 1
}" > /dev/null || true

# Task 3: No-estimate task
curl -s -X POST "$API_URL/api/v1/tasks" "${headers[@]}" -d "{
  \"name\": \"Brainstorm architecture ideas\",
  \"estimated_mins\": null,
  \"scheduled_on\": \"$today\",
  \"priority\": 0
}" > /dev/null || true

# Start task 1 so there is an active running timer
if [[ -n "$t1_id" ]]; then
  echo "Starting task #$t1_id..."
  curl -s -X POST "$API_URL/api/v1/tasks/$t1_id/start" "${headers[@]}" > /dev/null || true
fi

echo "Demo seed complete!"
