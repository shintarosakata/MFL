; ModuleID = 'main'
source_filename = "main"

define double @main() {
entry:
  %c = alloca double
  %b = alloca double
  %a = alloca double
  store double 5.000000e+00, double* %a
  store double 1.000000e+01, double* %b
  store double 2.000000e+01, double* %c
  ret double 0.000000e+00
}
