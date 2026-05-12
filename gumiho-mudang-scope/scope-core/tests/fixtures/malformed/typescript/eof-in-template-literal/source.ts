export function buildHtml(name: string): string {
    return `
        <html>
            <body>
                <p>Hello, ${name}</p>
            </body>

export function rebuild(): string {
    return buildHtml("world");
}
