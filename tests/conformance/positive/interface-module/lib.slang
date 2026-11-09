module lib;
public interface IValue { __init(int input); int read(); }
public int read<T : IValue>(T item) { return item.read(); }
public int create<T : IValue>(int input) { T item = T(input); return item.read(); }
