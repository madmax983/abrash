with open('examples/kintsugi_demo.rs', 'r') as f:
    text = f.read()
text = text.replace("fn main() -> Result<(), AppError> {", "fn main() -> Result<(), HostError> {")
with open('examples/kintsugi_demo.rs', 'w') as f:
    f.write(text)
