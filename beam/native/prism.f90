! prism.f90 — A Prism IS a projection matrix.
!
! preview = project + check nonzero.
! review  = embed via transpose.
! modify  = complement + transform.
! compose = matmul.
!
! The routing IS the computation.

module optics_prism
  use iso_c_binding
  implicit none

  private
  public :: prism_preview, prism_review, prism_modify, prism_compose

  real(c_double), parameter :: EPS = 1.0d-12

contains

  ! P * source = focus. matched = 1 if ||focus|| > eps, else 0.
  subroutine prism_preview(n, projection, source, focus, matched) &
      bind(c, name="prism_preview")
    integer(c_int), value, intent(in) :: n
    real(c_double), intent(in) :: projection(n, n)
    real(c_double), intent(in) :: source(n)
    real(c_double), intent(out) :: focus(n)
    integer(c_int), intent(out) :: matched

    real(c_double) :: norm
    integer :: i

    ! Project source into subspace
    focus = matmul(projection, source)

    ! Check if anything landed — L2 norm
    norm = 0.0d0
    do i = 1, n
      norm = norm + focus(i) * focus(i)
    end do
    norm = sqrt(norm)

    if (norm > EPS) then
      matched = 1
    else
      matched = 0
    end if
  end subroutine prism_preview

  ! P^T * focus = result (embed from subspace into full space)
  subroutine prism_review(n, projection, focus, result) &
      bind(c, name="prism_review")
    integer(c_int), value, intent(in) :: n
    real(c_double), intent(in) :: projection(n, n)
    real(c_double), intent(in) :: focus(n)
    real(c_double), intent(out) :: result(n)

    ! Embed via transpose — P^T maps from subspace back to full space
    result = matmul(transpose(projection), focus)
  end subroutine prism_review

  ! (I - P) * source + T * (P * source)
  ! Keep the complement unchanged, transform the matched part.
  subroutine prism_modify(n, projection, source, transform, result) &
      bind(c, name="prism_modify")
    integer(c_int), value, intent(in) :: n
    real(c_double), intent(in) :: projection(n, n)
    real(c_double), intent(in) :: source(n)
    real(c_double), intent(in) :: transform(n, n)
    real(c_double), intent(out) :: result(n)

    real(c_double) :: projected(n), complement(n), identity(n, n)
    integer :: i

    ! Build identity
    identity = 0.0d0
    do i = 1, n
      identity(i, i) = 1.0d0
    end do

    ! Project into subspace
    projected = matmul(projection, source)

    ! Complement: (I - P) * source
    complement = matmul(identity - projection, source)

    ! Result: complement + T * projected
    result = complement + matmul(transform, projected)
  end subroutine prism_modify

  ! composed = P2 * P1 (matrix multiply — prism composition)
  subroutine prism_compose(n, p1, p2, composed) &
      bind(c, name="prism_compose")
    integer(c_int), value, intent(in) :: n
    real(c_double), intent(in) :: p1(n, n)
    real(c_double), intent(in) :: p2(n, n)
    real(c_double), intent(out) :: composed(n, n)

    composed = matmul(p2, p1)
  end subroutine prism_compose

end module optics_prism
