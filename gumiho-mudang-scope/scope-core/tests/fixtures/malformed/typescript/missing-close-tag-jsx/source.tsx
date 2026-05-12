import * as React from "react";

export function Greeter(props: { name: string }) {
    return (
        <div className="container">
            <p>Hello, {props.name}</p>

    );
}

export function Farewell() {
    return <span>Bye</span>;
}
