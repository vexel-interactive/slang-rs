module lib;
public T copy<T>(T value) { return value; }
public T relay<T>(T value) { return copy(value); }
public T create<T>(T value = relay(T())) { return value; }
