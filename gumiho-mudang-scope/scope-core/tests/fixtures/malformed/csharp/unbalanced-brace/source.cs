namespace Acme.Example;

public class Inventory
{
    public int Count;

    public void Restock()
    {
        Count += 10;

    public void Drain()
    {
        Count = 0;
    }
}
