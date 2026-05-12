export class Inventory {
    count = 0;

    restock(amount: number): void {
        this.count += amount;

    drain(): void {
        this.count = 0;
    }
}
