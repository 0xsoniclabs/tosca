#if EVMZERO_ENABLE_MIMALLOC

#include <mimalloc.h>

// replaceable allocation functions

[[nodiscard]] void* operator new(size_t size) {
  void* ptr = mi_new(size);
  return ptr;
}

[[nodiscard]] void* operator new[](size_t size) {
  void* ptr = mi_new(size);
  return ptr;
}

[[nodiscard]] void* operator new(size_t size, std::align_val_t al) {
  void* ptr = mi_new_aligned(size, static_cast<size_t>(al));
  return ptr;
}

[[nodiscard]] void* operator new[](size_t size, std::align_val_t al) {
  void* ptr = mi_new_aligned(size, static_cast<size_t>(al));
  return ptr;
}

// replaceable non-throwing allocation functions

[[nodiscard]] void* operator new(size_t size, const std::nothrow_t&) noexcept {
  void* ptr = mi_new_nothrow(size);
  return ptr;
}

[[nodiscard]] void* operator new[](size_t size, const std::nothrow_t&) noexcept {
  void* ptr = mi_new_nothrow(size);
  return ptr;
}

[[nodiscard]] void* operator new(size_t size, std::align_val_t al, const std::nothrow_t&) noexcept {
  void* ptr = mi_new_aligned_nothrow(size, static_cast<size_t>(al));
  return ptr;
}

[[nodiscard]] void* operator new[](size_t size, std::align_val_t al, const std::nothrow_t&) noexcept {
  void* ptr = mi_new_aligned_nothrow(size, static_cast<size_t>(al));
  return ptr;
}

// replaceable usual deallocation functions

void operator delete(void* ptr) noexcept { mi_free(ptr); }

void operator delete[](void* ptr) noexcept { mi_free(ptr); }

void operator delete(void* ptr, std::align_val_t al) noexcept { mi_free_aligned(ptr, static_cast<size_t>(al)); }

void operator delete[](void* ptr, std::align_val_t al) noexcept { mi_free_aligned(ptr, static_cast<size_t>(al)); }

void operator delete(void* ptr, size_t) noexcept { mi_free(ptr); }

void operator delete[](void* ptr, size_t) noexcept { mi_free(ptr); }

void operator delete(void* ptr, size_t, std::align_val_t al) noexcept { mi_free_aligned(ptr, static_cast<size_t>(al)); }

void operator delete[](void* ptr, size_t, std::align_val_t al) noexcept {
  mi_free_aligned(ptr, static_cast<size_t>(al));
}

// replaceable placement deallocation functions

void operator delete(void* ptr, const std::nothrow_t&) noexcept { mi_free(ptr); }

void operator delete[](void* ptr, const std::nothrow_t&) noexcept { mi_free(ptr); }

void operator delete(void* ptr, std::align_val_t al, const std::nothrow_t&) noexcept {
  mi_free_aligned(ptr, static_cast<size_t>(al));
}

void operator delete[](void* ptr, std::align_val_t al, const std::nothrow_t&) noexcept {
  mi_free_aligned(ptr, static_cast<size_t>(al));
}

#endif
