namespace Acme.Example;

public class OrderProcessor
{
    public int Total { get; set; }

    public int ComputeTotal(int[] items)
    {
        int sum = 0;
        foreach (var item in items)
        {
            sum +=
