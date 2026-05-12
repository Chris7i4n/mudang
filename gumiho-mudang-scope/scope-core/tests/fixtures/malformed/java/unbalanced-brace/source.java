package com.acme.example;

public class Inventory {
    private int count;

    public void restock() {
        count += 10;

    public void drain() {
        count = 0;
    }
}
