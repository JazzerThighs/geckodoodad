INTERFACE FOR SEARCHING THROUGH PARSED JSON OF GECKO CODES: https://codepen.io/JazzerThighs/pen/xxmwypy

Beginner project for learning Rust (ChatGPT wrote it lmao): geckodoodad

*Goal: Extract all of the gecko codes from a text file and compile them, adding categorization based on mem-address references.

The largest collection of gecko codes is @ https://wiki.supercombo.gg/w/SSBM/Gecko_Codes

Regular Expressions curtesy of @ribbanya:
https://github.com/MeleeWorkshop/wiki2gecko <= Being used as a general guide

Inside ```fn extract_opcode_and_address```, there is a ```match``` statement containing all of the opcodes that are detected when trying to find a memory address reference inside a gecko code.

NOTE: When updating the raw .md file from the Wiki, use ```Shift```+```Tab``` on the whole document to remove the leading whitespace from every line of text.

