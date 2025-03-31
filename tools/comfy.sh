# Be sure you have loaded the virtual environment for ComfyUI
# before running this script, 
# source ../ComfyUI/.venv/bin/activate

# start comfyui but override the output directory to make life easier
python ../ComfyUI/main.py  --output-directory art/output --user-directory art/user --input-directory art/input 
# --lowvram --verbose DEBUG