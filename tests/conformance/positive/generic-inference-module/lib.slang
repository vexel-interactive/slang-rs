module lib;
public T copy<T>(T value) { return value; }
public T relay<T>(T value) { return copy(value); }
public T choose<T>(T value) { return value; }
public int choose(int value) { return value + 1; }
