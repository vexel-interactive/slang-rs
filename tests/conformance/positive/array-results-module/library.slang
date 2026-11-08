module library;
public typedef float Values[3];
public Values copy(Values values) { return values; }
public Values load(ConstantBuffer<Values> buffer) { return buffer; }
