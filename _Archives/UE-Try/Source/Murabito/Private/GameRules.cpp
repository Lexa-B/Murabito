#include "GameRules.h"

#include "CameraRig.h"
#include "InputController.h"
#include "MurabitoLog.h"

AGameRules::AGameRules()
{
	DefaultPawnClass = ACameraRig::StaticClass();
	PlayerControllerClass = AInputController::StaticClass();
}

void AGameRules::BeginPlay()
{
	Super::BeginPlay();
	UE_LOG(LogMurabito, Log, TEXT("GameRules: started"));
}
