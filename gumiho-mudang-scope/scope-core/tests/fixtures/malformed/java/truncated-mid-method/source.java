package com.acme.example;

public class OrderProcessor {
    private int total;

    public int computeTotal(int[] items) {
        int sum = 0;
        for (int item : items) {
            sum +=
