export class OrderProcessor {
    total = 0;

    computeTotal(items: number[]): number {
        let sum = 0;
        for (const item of items) {
            sum +=
