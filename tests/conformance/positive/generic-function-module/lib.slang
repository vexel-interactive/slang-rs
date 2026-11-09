module lib;
public T copy<T>(T value) { return value; }
public T relay<T>(T value) { return copy<T>(value); }
