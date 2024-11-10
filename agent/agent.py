import json
import requests

# Initialize collected information
background = None
body_color = None
eyes = None
special_traits = None
account_id = None

def get_missing_info(account_id, background, body_color, eyes, special_traits):
    missing_items = []
    if not account_id:
        missing_items.append("NEAR account address")
    if not background:
        missing_items.append("background (a short description of the environment where you want your dragon to be, for example, 'forest' or 'cave')")
    if not body_color:
        missing_items.append("body color (for example, 'green')")
    if not eyes:
        missing_items.append("eye color (for example, 'red')")
    # Optional, don't require this if everything else is provided
    if not special_traits and len(missing_items) > 0:
        missing_items.append("special traits (optional, can give your dragon a unique feature, for example, 'casting a water magic spell')")
    return missing_items

def prompt_user_with_message(message):
    env.add_message("agent", message)
    env.request_user_input()

def start_minting():
    global account_id, background, body_color, eyes, special_traits

    while True:
        try:
            extraction_prompt = {
                "role": "system",
                "content": """From the user's response, extract any information about background, body color, eyes, and special_traits fields.
                Provide the extracted information in JSON format with keys 'account_id', 'background', 'body_color', 'eyes', and 'special_traits'.
                The 'account_id' is the NEAR account ID of the user who wants to mint their NFT, usually ends in .near or .tg, but can also be 64 hexadecimal characters or a 0x-prefixed hexadecimal string like Ethereum addresses.
                The 'background' must be a short description of the background (from 1 to 8 words), 'body_color' and 'eyes' are colors (1-3 words), 'special_traits' is an optional free field up to 50 characters, or "dragon" if not provided.
                If any information is missing, omit that key in the JSON. If there's no information, return an empty JSON object."""
            }
            extraction_response = env.completion(env.list_messages() + [extraction_prompt])
            extraction_response = env.completion([
                {
                    "role": "user",
                    "content": extraction_response
                },
                {
                    "role": "system",
                    "content": "You are an AI assistant that helps find a JSON object in another AI's message. Your job is to extract the JSON object from the user's response and return it as a JSON object. If the user's response is not a valid JSON object, return an empty JSON object. Don't write any text before or after the JSON, and don't format your response, return plain JSON. Make sure to include absolutely no additional words, and the entire response is ready to be parsed as a JSON object, remove all comments, formatting, leave only the raw JSON."
                }
            ])

            # Try to parse the JSON
            try:
                extracted_info = json.loads(extraction_response)
            except json.JSONDecodeError:
                # Handle parsing error, ask the user again
                prompt_user_with_message("Sorry, I couldn't understand your response. Could you please provide the information again? Here's what I have:\n" + extraction_response)
                break

            # Update the collected information
            if 'account_id' in extracted_info:
                account_id = extracted_info['account_id']
            if 'background' in extracted_info:
                background_candidate = extracted_info['background']
                if len(background_candidate.split()) <= 8:
                    background = background_candidate
                else:
                    prompt_user_with_message("Sorry, the background is too detailed. Please use up to 8 words to describe it.")
                    background = None
                    break
            if 'body_color' in extracted_info:
                body_color_candidate = extracted_info['body_color']
                if len(body_color_candidate.split()) <= 3:
                    body_color = body_color_candidate
                else:
                    prompt_user_with_message("Use up to 3 words to describe body color.")
                    body_color = None
                    break
            if 'eyes' in extracted_info:
                eyes_candidate = extracted_info['eyes']
                if len(eyes_candidate.split()) <= 3:
                    eyes = eyes_candidate
                else:
                    prompt_user_with_message("Use up to 3 words to describe eye color.")
                    eyes = None
                    break
            if 'special_traits' in extracted_info:
                special_traits = extracted_info['special_traits']

            # Check if all required information is collected
            missing_info = get_missing_info(account_id, background, body_color, eyes, special_traits)
            if len(missing_info) > 0:
                # Ask for missing information
                if len(missing_info) == 1:
                    agent_message = f"Please provide your {missing_info[0]}."
                else:
                    agent_message = f"Please provide your {', '.join(missing_info[:-1])}, and {missing_info[-1]}."
                prompt_user_with_message(agent_message)
                break
            else:
                env.add_message("agent", "Generating and minting your NEARvember NFT...")
                image_url, token_id = mint(account_id, background, body_color, eyes, special_traits)
                env.write_file("index.html", env.read_file("template.html").replace("{{TOKEN_ID}}", token_id).replace("{{IMAGE_URL}}", image_url))
                env.mark_done()
                break
        except Exception as e:
            env.add_message("agent", "Error: " + str(e) + "\n\nPlease report this to https://t.me/slimedrgn")
            break

def mint(account_id, background, body_color, eyes, special_traits):
    url = "https://mint-agent.nearvember.xyz/generate"
    data = {
        "background": background,
        "body_color": body_color,
        "eyes": eyes,
        "special_traits": special_traits,
        "account_id": account_id
    }
    response = requests.post(url, json=data)
    if response.status_code == 200:
        response_json = response.json()
        return response_json["image_url"], response_json["token_id"]
    else:
        env.add_message("agent", "Error: Failed to generate or mint NFT, send this to t.me/slimedrgn:" + response.text)
    # return "https://fal.media/files/kangaroo/bEHPPdhdt9T0l6a0VNfWS_44cce91f34fe4bb79a1a10e0e32b5f74.png", "0"

# Main flow
question_for_llm =  {"role": "system", "content": "Did the user ask to mint an NFT just now? Answer with only 'yes' or 'no'"}
did_user_ask_to_mint = env.completion(env.list_messages() + [question_for_llm]).replace("\n", " ")

if "yes" in did_user_ask_to_mint.lower():
    start_minting()
else:
    agent_message = "Hey, I've heard it's NEARvember! What do you want to do? Perhaps mint something for free?"
    env.add_message("agent", agent_message)
